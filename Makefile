# Load environment variables from .env
ifneq (,$(wildcard ./.env))
	include .env
	export
endif

# Export default environment variables
## NOTE: you should only change these for you local environment. For other
## environments like beta or prod, set the DATABASE_URL in your .env file.
DB_USER ?= postgres
DB_PASS ?= password
DB_NAME ?= racer
DB_HOST ?= localhost
DB_PORT ?= 5432
DATABASE_URL ?= postgres://${DB_USER}:${DB_PASS}@${DB_HOST}:${DB_PORT}/${DB_NAME}
DOCKER_CONTAINER_NAME ?= racer-database

_TITLE := "\033[32m[%s]\033[0m %s\n" # Green text for "printf"
_ERROR := "\033[31m[%s]\033[0m %s\n" # Red text for "printf"

# Hook to check if command exists
cmd-exists-%:
	@hash $(*) > /dev/null 2>&1 || \
		(echo "ERROR: '$(*)' must be installed and available on your PATH."; exit 1)

# Hook to wait for Postgres to become available
wait-for-postgres: cmd-exists-psql
	@until psql ${DATABASE_URL} -c '\q'; do \
		echo "Postgres is unavailable - trying again"; \
		sleep 1; \
	done
	@echo "Postgres is up and running on port ${DB_PORT}!"

# Non-user-facing command to initialize the postgres container
db-init: cmd-exists-docker
	docker run \
		--name="${DOCKER_CONTAINER_NAME}" \
		-e POSTGRES_USER="${DB_USER}" \
		-e POSTGRES_PASSWORD="${DB_PASS}" \
		-e POSTGRES_DB="${DB_NAME}" \
		-p "${DB_PORT}":5432 \
		-d postgres:14-alpine \
		postgres -N 1000

# If the CI variable is set, it automatically continues
.PHONY: confirm
confirm:
	@if [[ -z "$(CI)" ]]; then \
		REPLY="" ; \
		read -p "⚠ Are you sure? [y/n] > " -r ; \
		if [[ ! $$REPLY =~ ^[Yy]$$ ]]; then \
			printf $(_ERROR) "NO" "Stopping" ; \
			exit 1 ; \
		else \
			printf $(_TITLE) "OK" "Continuing" ; \
			exit 0; \
		fi \
	fi

# User-facing command to create the local database
.PHONY: db-create
db-create: db-init wait-for-postgres cmd-exists-sqlx
	@sqlx database create -D "${DATABASE_URL}"

# User-facing command to remove the local database
.PHONY: db-drop
db-drop: cmd-exists-docker
	@docker kill ${DOCKER_CONTAINER_NAME}
	@docker rm ${DOCKER_CONTAINER_NAME}

# User-facing command to run migrations
.PHONY: db-migrate
db-migrate: cmd-exists-sqlx
	@sqlx migrate run --dry-run
	@echo Are you sure you want to run this on ${DATABASE_URL}?
	@if $(MAKE) -s confirm ; then \
		sqlx migrate run; \
	fi

# User-facing command to revert migrations
.PHONY: db-revert
db-revert: cmd-exists-sqlx
	@sqlx migrate revert --dry-run
	@echo Are you sure you want to run this on ${DATABASE_URL}?
	@if $(MAKE) -s confirm ; then \
		sqlx migrate revert; \
	fi

# User-facing command to show the current connection string
.PHONY: db-show
db-show:
	@echo ${DATABASE_URL}

# User-facing command to quickly reset the local database
.PHONY: db-reset
db-reset: db-drop db-create db-migrate
	@echo The database has been reset!
