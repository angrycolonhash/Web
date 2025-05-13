use crate::database::Database;

mod database;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let conn = Database::init_db().await?;

    let tx = Database::start_transaction(&conn).await?;
    {
        
    }
    Database::commit_transaction(tx).await?;

    Ok(())
}