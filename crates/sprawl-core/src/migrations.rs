use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MigrationContext {
    pub current_version: u32,
    pub target_version: u32,
}

pub trait Migration {
    fn up(&self, context: &MigrationContext) -> Result<(), String>;
    fn down(&self, context: &MigrationContext) -> Result<(), String>;
    fn version(&self) -> u32;
}

pub struct MigrationManager {
    migrations: Vec<Box<dyn Migration>>,
}

impl Default for MigrationManager {
    fn default() -> Self {
        Self::new()
    }
}

impl MigrationManager {
    pub fn new() -> Self {
        Self {
            migrations: Vec::new(),
        }
    }

    pub fn register(&mut self, migration: Box<dyn Migration>) {
        self.migrations.push(migration);
        self.migrations.sort_by_key(|m| m.version());
    }

    pub fn migrate_up(&self, context: &mut MigrationContext) -> Result<(), String> {
        for migration in &self.migrations {
            if migration.version() > context.current_version && migration.version() <= context.target_version {
                migration.up(context)?;
                context.current_version = migration.version();
            }
        }
        Ok(())
    }
}
