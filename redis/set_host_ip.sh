IP=`ifconfig eth0 2>/dev/null | grep -Eo 'inet (addr:)?([0-9]*\.){3}[0-9]*' | grep -Eo '([0-9]*\.){3}[0-9]*'`
if [ -z "$IP" ]; then
  IP=`hostname -I | awk '{print $1}'`
fi
echo IP=$IP
echo "HOST_IP=$IP" > .env
