#[cfg(feature = "real-archivist")]
pub mod lancedb_backend {
    use crate::{IndexedChunk, SearchResult, VectorDatabase, Result, ArchivistError};
    use lancedb::{connect, Table};
    use lancedb::query::{ExecutableQuery, QueryBase};
    use lancedb::index::Index;
    use std::sync::Arc;
    use arrow_schema::{Schema, Field, DataType};
    use arrow_array::{
        RecordBatch, StringArray, UInt32Array, FixedSizeListArray,
        cast::AsArray, types::Float32Type,
    };
    use futures::TryStreamExt;

    pub struct LanceVectorDb {
        table: Table,
    }

    impl LanceVectorDb {
        pub async fn connect(path: &str) -> Result<Self> {
            let conn = connect(path).execute().await
                .map_err(|e| ArchivistError::Database(e.to_string()))?;

            let schema = Arc::new(Schema::new(vec![
                Field::new("id", DataType::Utf8, false),
                Field::new("project_id", DataType::Utf8, false),
                Field::new("file_path", DataType::Utf8, false),
                Field::new("chunk_text", DataType::Utf8, false),
                Field::new("start_line", DataType::UInt32, false),
                Field::new("end_line", DataType::UInt32, false),
                Field::new(
                    "vector",
                    DataType::FixedSizeList(
                        Arc::new(Field::new("item", DataType::Float32, true)),
                        384,
                    ),
                    false,
                ),
            ]));

            let table_names = conn.table_names().execute().await
                .map_err(|e| ArchivistError::Database(e.to_string()))?;

            let table = if table_names.contains(&"sprawl_chunks".to_string()) {
                conn.open_table("sprawl_chunks").execute().await
                    .map_err(|e| ArchivistError::Database(e.to_string()))?
            } else {
                let tbl = conn.create_empty_table("sprawl_chunks", schema).execute().await
                    .map_err(|e| ArchivistError::Database(e.to_string()))?;
                // Best-effort: create ANN index (no-op on empty table, won't fail)
                let _ = tbl.create_index(&["vector"], Index::Auto).execute().await;
                tbl
            };

            Ok(Self { table })
        }
    }

    #[async_trait::async_trait]
    impl VectorDatabase for LanceVectorDb {
        async fn search(&self, query_embedding: &[f32], top_k: usize) -> Result<Vec<SearchResult>> {
            let mut stream = self.table
                .query()
                .nearest_to(query_embedding)
                .map_err(|e| ArchivistError::Database(format!("ANN query build failed: {e}")))?
                .limit(top_k)
                .execute()
                .await
                .map_err(|e| ArchivistError::Database(e.to_string()))?;

            let mut results = Vec::new();
            while let Some(batch) = stream.try_next().await
                .map_err(|e| ArchivistError::Database(e.to_string()))?
            {
                let project_id_col = batch.column_by_name("project_id")
                    .ok_or_else(|| ArchivistError::Database("missing column: project_id".into()))?
                    .as_string::<i32>();
                let file_path_col = batch.column_by_name("file_path")
                    .ok_or_else(|| ArchivistError::Database("missing column: file_path".into()))?
                    .as_string::<i32>();
                let chunk_text_col = batch.column_by_name("chunk_text")
                    .ok_or_else(|| ArchivistError::Database("missing column: chunk_text".into()))?
                    .as_string::<i32>();
                let start_line_col = batch.column_by_name("start_line")
                    .ok_or_else(|| ArchivistError::Database("missing column: start_line".into()))?
                    .as_primitive::<arrow_array::types::UInt32Type>();
                let end_line_col = batch.column_by_name("end_line")
                    .ok_or_else(|| ArchivistError::Database("missing column: end_line".into()))?
                    .as_primitive::<arrow_array::types::UInt32Type>();
                let distance_col = batch.column_by_name("_distance")
                    .ok_or_else(|| ArchivistError::Database("missing column: _distance".into()))?
                    .as_primitive::<arrow_array::types::Float32Type>();

                for i in 0..batch.num_rows() {
                    results.push(SearchResult {
                        project_id: project_id_col.value(i).to_string(),
                        file_path: file_path_col.value(i).to_string(),
                        chunk_text: chunk_text_col.value(i).to_string(),
                        start_line: start_line_col.value(i),
                        end_line: end_line_col.value(i),
                        // L2 distance: convert to similarity score in [0, 1]
                        similarity_score: 1.0 / (1.0 + distance_col.value(i)),
                    });
                }
            }

            Ok(results)
        }

        async fn insert(&self, chunks: &[IndexedChunk]) -> Result<()> {
            if chunks.is_empty() {
                return Ok(());
            }

            let ids: Vec<Option<&str>> = chunks.iter().map(|c| Some(c.id.as_str())).collect();
            let project_ids: Vec<Option<&str>> = chunks.iter().map(|c| Some(c.project_id.as_str())).collect();
            let file_paths: Vec<Option<&str>> = chunks.iter().map(|c| Some(c.file_path.as_str())).collect();
            let chunk_texts: Vec<Option<&str>> = chunks.iter().map(|c| Some(c.chunk_text.as_str())).collect();
            let start_lines: Vec<Option<u32>> = chunks.iter().map(|c| Some(c.chunk_start_line)).collect();
            let end_lines: Vec<Option<u32>> = chunks.iter().map(|c| Some(c.chunk_end_line)).collect();

            let id_array = StringArray::from(ids);
            let project_id_array = StringArray::from(project_ids);
            let file_path_array = StringArray::from(file_paths);
            let chunk_text_array = StringArray::from(chunk_texts);
            let start_line_array = UInt32Array::from(start_lines);
            let end_line_array = UInt32Array::from(end_lines);

            // Build a flat f32 buffer with all embeddings, each padded/truncated to 384 dims
            let vector_array = FixedSizeListArray::from_iter_primitive::<Float32Type, _, _>(
                chunks.iter().map(|c| {
                    let mut emb = c.embedding.clone();
                    emb.resize(384, 0.0);
                    Some(emb.into_iter().map(Some))
                }),
                384,
            );

            let schema = self.table.schema().await
                .map_err(|e| ArchivistError::Database(e.to_string()))?;

            let batch = RecordBatch::try_new(
                Arc::new(schema.as_ref().clone()),
                vec![
                    Arc::new(id_array),
                    Arc::new(project_id_array),
                    Arc::new(file_path_array),
                    Arc::new(chunk_text_array),
                    Arc::new(start_line_array),
                    Arc::new(end_line_array),
                    Arc::new(vector_array),
                ],
            ).map_err(|e| ArchivistError::Database(e.to_string()))?;

            self.table.add(vec![batch]).execute().await
                .map_err(|e| ArchivistError::Database(e.to_string()))?;

            Ok(())
        }
    }
}
