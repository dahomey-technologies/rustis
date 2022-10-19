FROM redis:alpine

RUN mkdir -p /redis

WORKDIR /redis

COPY sentinel.conf .

EXPOSE 26379

ENTRYPOINT ["redis-server", "/redis/sentinel.conf", "--sentinel"]