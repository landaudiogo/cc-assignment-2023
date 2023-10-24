FROM rust:1.73.0

ARG PACKAGE

WORKDIR /app

RUN apt-get update \
    && apt-get install -y cmake

ADD ./.sqlx ./.sqlx
COPY ./Cargo.toml .
COPY ./Cargo.lock .
ADD ./event-hash ./event-hash
ADD ./experiment-producer ./experiment-producer
ADD ./http-load-generator ./http-load-generator
ADD ./notifications-service ./notifications-service
ADD ./notifier ./notifier
ADD ./test-to-api ./test-to-api
# RUN git clone https://github.com/landaudiogo/cc-assignment-2023 .
RUN cargo install --path "./${PACKAGE}"

COPY --chmod=500 entrypoint.sh .

ENV PACKAGE="${PACKAGE}"

ENTRYPOINT ["./entrypoint.sh"]
