CREATE TABLE demo.notification (
    measurement_id TEXT, 
    group_id TEXT,
    latency DOUBLE PRECISION,
    PRIMARY KEY(measurement_id, group_id)
)
