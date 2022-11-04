#!/bin/sh
sed -i "s/\$PORT/$PORT/g" /redis/cluster.conf
redis-server /redis/cluster.conf
