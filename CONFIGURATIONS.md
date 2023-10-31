**prometheus**: 

- `/observability/.env`: Specifies demo service & student hosts. 
- `/observability/prometheus-config.yml`: Configures prometheus' targets.

**grafana**: 

- `/observability/grafana.env`: Admin credentials

**postgre**:

- `/database/.env`: Postgre admin credentials

**notifications-service**:

- `/.env`: 
    - NOTIFICATIONS_EXTERNAL_IP
    - SECRET_KEY
- `/notifications-service/.env`
    - DATABASE_URL
    - SQLX_OFFLINE

**experiment-producer**:

- `/.env`: 
    - SECRET_KEY
    - BROKERS
    - TOPIC
    - TOPIC_DOCUMENT
- `/experiment-producer/auth` 
- `/experiment-producer/.env`
    - DATABASE_URL
    - SQLX_OFFLINE
- `/experiment-produecr/config.json`

**http-load-generator**: 

- `/.env`:
    - BROKERS
    - TOPIC_DOCUMENT
    - HTTP_CONSUMER_GROUP
    - HTTP_CONSUMER_WAIT_BEFORE_SEND
    - HTTP_MIN_BATCH_SIZE
    - HTTP_MAX_BATCH_SIZE
    - HTTP_NUM_GENERATIONS
    - HTTP_RETRIES
- `/auth`
- `http-load-generator/hosts.json`

**toapi**:

- `/.env`
    - SECRET_KEY
    - BROKERS
    - TOPIC_DOCUMENT
    - TOAPI_CONSUMER_GROUP
- `/auth`

**notifier**:

- `/.env`
    - SECRET_KEY
    - BROKERS
    - TOPIC
    - NOTIFIER_CONSUMER_GROUP
    - NOTIFIER_NOTIFICATIONS_HOST
- `/auth`
