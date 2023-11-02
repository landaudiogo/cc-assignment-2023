ENVIRONMENT="data-consistency"

SERVICES=-f docker-compose.services.yml
TO=-f docker-compose.to.yml

.PHONY=run_producer run_http all

all: 

run_producer:
	./scripts/setup_env.sh $(ENVIRONMENT)
	docker compose $(SERVICES) pull
	docker compose $(SERVICES) up -d experiment-producer

run_http: 
	./scripts/setup_env.sh $(ENVIRONMENT)
	docker compose $(SERVICES) pull
	docker compose $(SERVICES) up -d http-load-generator

run_to: 
	./scripts/setup_env.sh $(ENVIRONMENT)
	docker compose $(TO) pull
	docker compose $(TO) up -d
