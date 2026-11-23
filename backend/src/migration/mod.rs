pub use sea_orm_migration::prelude::*;

mod m20261123_000001_initial_schema;
mod m20261123_000002_drop_download_jobs_fk;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20261123_000001_initial_schema::Migration),
            Box::new(m20261123_000002_drop_download_jobs_fk::Migration),
        ]
    }
}
