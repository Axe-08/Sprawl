#[cfg(feature = "real-archivist")]
pub mod lancedb_backend {
    use sprawl_archivist::{IndexedChunk, SearchResult, VectorDatabase};
    use sprawl_core::Result;
    use lancedb::{Connection, Table};
    use std::sync::Arc;
    use arrow_schema::{Schema, Field, DataType};
    use arrow_array::{RecordBatch, StringArray, Float32Array, UInt32Array, FixedSizeListArray, cast::AsArray, types::Float32Type};
    // We will need futures for stream next
    use futures_util::StreamExt;

    pub struct LanceVectorDb {
        table: Arc<Table>,
    }

    impl LanceVectorDb {
        pub async fn connect(path: &str) -> Result<Self> {
            let conn = lancedb::connect(path).execute().await
                .map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?;

            let schema = Arc::new(Schema::new(vec![
                Field::new("id", DataType::Utf8, false),
                Field::new("project_id", DataType::Utf8, false),
                Field::new("file_path", DataType::Utf8, false),
                Field::new("chunk_text", DataType::Utf8, false),
                Field::new("start_line", DataType::UInt32, false),
                Field::new("end_line", DataType::UInt32, false),
                Field::new(
                    "item", // lancedb requires the vector column to be named 'vector' usually, but 'item' is used in schema? 
                    // Actually lancedb default is 'vector' or you can specify it. Let's use 'vector'
                    DataType::FixedSizeList(
                        Arc::new(Field::new("item", DataType::Float32, true)),
                        384,
                    ),
                    false,
                ),
            ]));

            let table_names = conn.table_names().execute().await
                .map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?;

            let table = if table_names.contains(&"sprawl_chunks".to_string()) {
                conn.open_table("sprawl_chunks").execute().await
                    .map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?
            } else {
                conn.create_empty_table("sprawl_chunks", schema).execute().await
                    .map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?
            };

            Ok(Self {
                table: Arc::new(table),
            })
        }
    }

    #[async_trait::async_trait]
    impl VectorDatabase for LanceVectorDb {
        async fn search(&self, query_embedding: &[f32], top_k: usize) -> sprawl_core::Result<Vec<SearchResult>> {
            let query_vec = lancedb::query::Query::new(self.table.clone())
                .column("vector")
                .limit(top_k)
                .execute()
                .await
                .map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?;

            let mut results = Vec::new();
            let mut stream = query_vec;
            while let Some(batch) = stream.next().await {
                let batch = batch.map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?;
                let project_id_col = batch.column_by_name("project_id").unwrap().as_string::<i32>();
                let file_path_col = batch.column_by_name("file_path").unwrap().as_string::<i32>();
                let chunk_text_col = batch.column_by_name("chunk_text").unwrap().as_string::<i32>();
                let start_line_col = batch.column_by_name("start_line").unwrap().as_primitive::<arrow_array::types::UInt32Type>();
                let end_line_col = batch.column_by_name("end_line").unwrap().as_primitive::<arrow_array::types::UInt32Type>();
                let distance_col = batch.column_by_name("_distance").unwrap().as_primitive::<arrow_array::types::Float32Type>();

                for i in 0..batch.num_rows() {
                    results.push(SearchResult {
                        project_id: project_id_col.value(i).to_string(),
                        file_path: file_path_col.value(i).to_string(),
                        chunk_text: chunk_text_col.value(i).to_string(),
                        start_line: start_line_col.value(i),
                        end_line: end_line_col.value(i),
                        similarity_score: 1.0 - distance_col.value(i), // Approx similarity
                    });
                }
            }

            Ok(results)
        }

        async fn insert(&self, chunks: &[IndexedChunk]) -> sprawl_core::Result<()> {
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

            let mut all_embeddings = Vec::with_capacity(chunks.len() * 384);
            for c in chunks {
                let mut emb = c.embedding.clone();
                if emb.len() != 384 {
                    emb.resize(384, 0.0);
                }
                all_embeddings.extend(emb);
            }
            
            let emb_array = Float32Array::from(all_embeddings);
            let field = Arc::new(Field::new("item", DataType::Float32, true));
            let fixed_size_list = FixedSizeListArray::try_new(
                field,
                384,
                Arc::new(emb_array),
                None,
            ).map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?;

            let batch = RecordBatch::try_new(
                self.table.schema().await.map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?,
                vec![
                    Arc::new(id_array),
                    Arc::new(project_id_array),
                    Arc::new(file_path_array),
                    Arc::new(chunk_text_array),
                    Arc::new(start_line_array),
                    Arc::new(end_line_array),
                    Arc::new(fixed_size_list),
                ],
            ).map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?;

            self.table.add(vec![batch]).execute().await
                .map_err(|e| sprawl_core::SprawlError::Other(e.to_string()))?;

            Ok(())
        }
    }
}
