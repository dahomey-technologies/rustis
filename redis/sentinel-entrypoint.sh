#!/bin/sh
sed -i "s/\$PORT/$PORT/g" /redis/sentinel.conf
sed -i "s/\$HOST_IP/$HOST_IP/g" /redis/sentinel.conf
redis-server /redis/sentinel.conf --sentinel
