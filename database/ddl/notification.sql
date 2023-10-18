DROP TABLE demo.notification;

CREATE TABLE demo.notification (
    experiment_id TEXT,
    measurement_id TEXT, 
    group_id TEXT,
    latency DOUBLE PRECISION,
    PRIMARY KEY(experiment_id, measurement_id, group_id)
);
