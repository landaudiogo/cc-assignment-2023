FROM rust:1.73.0

ARG PACKAGE

WORKDIR /app

RUN apt-get update \
    && apt-get install -y cmake

RUN git clone https://github.com/landaudiogo/cc-assignment-2023 .
RUN cargo install --path "./${PACKAGE}"

COPY --chmod=500 entrypoint.sh .

ENV PACKAGE="${PACKAGE}"

ENTRYPOINT ["./entrypoint.sh"]
