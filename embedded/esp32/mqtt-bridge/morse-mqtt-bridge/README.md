# Morse MQTT Bridge

This is an ESP32 based morse code transceiver and MQTT bridge.


## Deploy MQTT server

You need an MQTT server to test with. You can use test.mosquitto.org
and [generate a test certificate](https://test.mosquitto.org/ssl/).

You can deploy your own [self-hosted mosquitto server with
d.rymcg.tech](https://github.com/EnigmaCurry/d.rymcg.tech/tree/master/mosquitto#readme).


## Set secrets.h

Copy [secrets.example.h](secrets.example.h) to `secrets.h` and edit
your Wifi and MQTT connection details.

## Deploy ESP32

description TODO

## Test MQTT client

Create two bash aliases to test easily:

 * `morse-recv` - subscribes to the MQTT bridge to receive messages.
 * `morse-send` - publish a message to the MQTT bridge to send a message.

```
# Put this in ~/.bashrc or similar


HOST=mqtt.example.com
CN=foo.clients.mqtt.example.com
CA_CERT=root_ca.crt
CERT=${CN}.crt
KEY=${CN}.key
PORT=8883
TOPIC=morse-bridge/rx_message

alias morse-recv="mosquitto_sub \
  -h ${HOST} \
  --cert $(realpath ${CERT}) \
  --key $(realpath ${KEY}) \
  --cafile $(realpath ${CA_CERT}) \
  -p ${PORT} \
  -t ${TOPIC}"
  
alias morse-send="mosquitto_pub \
  -h ${HOST} \
  --cert $(realpath ${CERT}) \
  --key $(realpath ${KEY}) \
  --cafile $(realpath ${CA_CERT}) \
  -p ${PORT} \
  -t ${TOPIC} \
  -m"
```


Open two terminals, run `morse-recv` in one and `morse-send` in another.

```
# morse-send "hello world"

# morse-recv
HELLO WORLD
```

