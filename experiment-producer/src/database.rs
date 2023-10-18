use sqlx::{Pool, Postgres};

pub async fn insert_ground_truth(
    pool: &Pool<Postgres>,
    experiment_id: &str,
    measurement_id: &str,
) -> Result<(), sqlx::Error> {
    sqlx::query!(
        "
            INSERT INTO 
                demo.notification_ground_truth (experiment_id, measurement_id) 
            VALUES 
                ($1, $2)
            ON CONFLICT
                DO NOTHING;
            ",
        experiment_id,
        measurement_id,
    )
    .execute(pool)
    .await?;

    Ok(())
}
