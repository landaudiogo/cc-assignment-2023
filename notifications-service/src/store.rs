use sqlx::{Pool, Postgres};

pub async fn insert_latency(
    pool: &Pool<Postgres>,
    group_id: &str,
    measurement_id: &str,
    latency: &f64,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "
            INSERT INTO 
                demo.notification (measurement_id, group_id, latency) 
            VALUES 
                ($1, $2, $3)
            ON CONFLICT
                DO NOTHING;
            ",
        group_id,
        measurement_id,
        latency
    )
    .execute(pool)
    .await?;

    Ok(())
}
