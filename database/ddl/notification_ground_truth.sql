DROP TABLE demo.notification_ground_truth;

CREATE TABLE demo.notification_ground_truth (
    experiment_id TEXT,
    measurement_id TEXT, 
    insert_timestamp TIMESTAMP DEFAULT now(),
    PRIMARY KEY(experiment_id, measurement_id)
);
