CREATE USER grafanareader WITH PASSWORD '***';

GRANT USAGE ON SCHEMA demo TO grafanareader;

GRANT SELECT
ON ALL TABLES IN SCHEMA demo
TO grafanareader;
