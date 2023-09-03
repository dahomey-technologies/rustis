#!/bin/sh
sed -i "s/\$PORT/$PORT/g" /redis/cluster.conf
sed -i "s/\$BUS_PORT/$BUS_PORT/g" /redis/cluster.conf
sed -i "s/\$HOST_IP/$HOST_IP/g" /redis/cluster.conf
redis-server /redis/cluster.conf
