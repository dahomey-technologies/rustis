FROM redis:alpine

RUN mkdir -p /redis

WORKDIR /redis

COPY sentinel.conf .

EXPOSE 26379

COPY sentinel-entrypoint.sh .

ENTRYPOINT ["/redis/sentinel-entrypoint.sh"]