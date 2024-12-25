FROM redis:alpine

WORKDIR /redis
COPY cluster.conf .
RUN chown redis:redis /redis/cluster.conf
EXPOSE 6379
COPY cluster-entrypoint.sh .
ENTRYPOINT ["/redis/cluster-entrypoint.sh"]
