// ESP32 Morse Code MQTT transceiver  

// Set your WiFi credentials and MQTT settings in secrets.h (see secrets.example.h)
// This controller will publish to the following MQTT topics based on your root topic name:
///  {MQTT_TOPIC_ROOT}/rx_stream    - the stream of morse code letters received
///  {MQTT_TOPIC_ROOT}/rx_message  - complete messages received split by prosign or timeout
//  {MQTT_TOPIC_ROOT}/tx_state  - streams the state of the tramsmit mode (1=tx 0=rx)
//  {MQTT_TOPIC_ROOT}/tx_stream  - the stream of morse code letters being sent
//  {MQTT_TOPIC_ROOT}/tx_message  - complete messages to send

#include "Arduino.h"
#include <WiFi.h>
#include "ESP32MQTTClient.h"
#include "secrets.h"
#include "Lewis.h"
#include <cstdio>
#include <ctype.h>
#include "esp_timer.h"

const char* HOST_NAME = SECRET_HOST_NAME; // e.g. morse-bridge
const char* WIFI_SSID = SECRET_WIFI_SSID; // e.g. fbi-van-5
const char* WIFI_PASS = SECRET_WIFI_PASS; // e.g. hunter2
const char* MQTT_SERVER = SECRET_MQTT_SERVER; // e.g. mqtts://test.mosquitto.org:8884
const char* TLS_CERT = SECRET_TLS_CERT; // e.g. your cert from https://test.mosquitto.org/ssl/
const char* TLS_KEY = SECRET_TLS_KEY; // your TLS key
const char* TLS_CA_CERT = SECRET_TLS_CA_CERT; // the server's CA cert
const char* MQTT_TOPIC_ROOT = SECRET_MQTT_TOPIC_ROOT; // e.g. morse-bridge
const bool MQTT_ENABLE_STREAMING = false; // e.g. enable single letter streaming
const int MQTT_QOS = 2;

const int WPM = 20;
const bool MORSE_INTERRUPT = false;
const int MORSE_INTERRUPT_FREQUENCY = 100; //Hz
const int MORSE_RX_PIN = 4;
const int MORSE_TX_PIN = 2;
const int MORSE_MESSAGE_TIMEOUT = 15; // Seconds after which an incomplete message will finalize
const int BUFFER_SIZE = 500;
const bool MQTT_RETAIN_MESSAGES = false;

ESP32MQTTClient mqttClient;
Lewis Morse;
unsigned long lastReceivedTime = 0; // Tracks the last time a character was received
unsigned long lastTransmitTime = 0; // Tracks the last time a character was sent
unsigned long lastPublishStateTime = 0;
bool tx_state = false; // Track current transmit or receive state
char buffer[BUFFER_SIZE] = ""; // Buffer the current message
unsigned int bufferIndex = 0;           // Tracks the current position in the buffer

void setup()
{
  Serial.begin(115200);
  /* log_i(); */
  /* log_i("setup, ESP.getSdkVersion(): "); */
  /* log_i("%s", ESP.getSdkVersion()); */
  delay(500);
  connectToWifi();

  connectToMqtt();
  publishTxState();
  
  Morse.begin(MORSE_RX_PIN, MORSE_TX_PIN, WPM, MORSE_INTERRUPT);

  /* morseTimerSemaphore = xSemaphoreCreateBinary(); */
  /* morseTimer = timerBegin(1000000); // timer ticks at 1Mhz */
  /* timerAttachInterrupt(morseTimer, &onMorseTimer); */
  /* timerAlarm(morseTimer, (1.0 / MORSE_INTERRUPT_FREQUENCY) * 1e6, true, 0); */
}

bool is_transmitting() {
  // longest word is fifteen dots, plus a three dot gap:
  bool is_tx = millis() - lastTransmitTime < (1200/WPM) * 18;
  // publish tx_state changes:
  if ((is_tx && !tx_state) || (!is_tx && tx_state)) {
    tx_state = is_tx;
    // publish instantaenous state change:
    publishTxState();
  } else if (!tx_state && (millis() - lastPublishStateTime > MORSE_MESSAGE_TIMEOUT * 1000)) {
    // publish periodic message that we're ready to receive:
    if (mqttClient.isConnected())
      publishTxState();
  }
  return tx_state;
}

void loop() {
  Morse.checkIncoming();

  if (!is_transmitting() && Morse.available()) {
    int inByte = (char)toupper(Morse.read());
    Serial.write(inByte);
    lastReceivedTime = millis();

    publishRxStream(inByte);
    
    // Check for buffer overflow
    if (bufferIndex >= BUFFER_SIZE - 1) {
      publishMessage();
      clearBuffer();
    }
    
    buffer[bufferIndex++] = (char)inByte;
    buffer[bufferIndex] = '\0'; // Null-terminate the string

    split_message_on_prosign(inByte, false);
  }
  // Check if an incomplete message has been idle and finalize the message log.
  if (bufferIndex > 0 && (millis() - lastReceivedTime > MORSE_MESSAGE_TIMEOUT * 1000)) {
    publishMessage();
    clearBuffer();
    publishMessageBreak();
    Serial.write("\n\n");
  }
  
  // Send each serial byte to Morse output
  if (Serial.available()) {
    int inByte = (char)toupper(Serial.read());
    Serial.write((char)inByte);
    Morse.write(inByte);
    lastTransmitTime = millis();
  }
  
  delay(10);
}

void connectToWifi() {
  Serial.print("# Conneting to WiFi ");
  Serial.println(SECRET_WIFI_SSID);
  WiFi.begin(SECRET_WIFI_SSID, WIFI_PASS);
  WiFi.setHostname(HOST_NAME);

  int t = 0;
  while (WiFi.status() != WL_CONNECTED) {
    if (t % 75 == 0) {
      Serial.print("\n# Still trying to connect to Wi-Fi ");
      Serial.println(SECRET_WIFI_SSID);
    }
    delay(500);
    Serial.print(".");
    t += 1;
  }

  Serial.println("\n# Connected to Wi-Fi!");
  Serial.print("# Hostname: ");
  Serial.println(HOST_NAME);
  Serial.print("# IP address: ");
  Serial.println(WiFi.localIP());
}

void connectToMqtt() {
  mqttClient.enableDebuggingMessages();

  mqttClient.setURI(SECRET_MQTT_SERVER);
  mqttClient.enableLastWillMessage("lwt", "I am going offline");
  mqttClient.setKeepAlive(30);
  mqttClient.setClientCert(TLS_CERT);
  mqttClient.setKey(TLS_KEY);
  mqttClient.setCaCert(TLS_CA_CERT);
  mqttClient.loopStart();

  int t = 0;
  while (true) {
    if (mqttClient.isConnected()) {
      Serial.write("# MQTT connected!\n");
      break;
    } else {
      if (t % 10 == 0) {
        Serial.write("# Waiting for MQTT connection ...\n");
      };
      t+=1;
      delay(500);
    }
  }
}

void onMqttConnect(esp_mqtt_client_handle_t client)
{
  if (mqttClient.isMyTurn(client)) // can be omitted if only one client
    {
      char topic[100];
      snprintf(topic, sizeof(topic), "%s/%s", MQTT_TOPIC_ROOT, "tx_message");
      mqttClient.subscribe(topic, [](const String &payload)
                           {
                             tx_state = true;
                             publishTxState();
                             for (size_t i = 0; i < payload.length(); i++)
                               {
                                 char inByte = (char)toupper(payload.charAt(i));
                                 // Check for buffer overflow:
                                 // TODO: fix long message truncation
                                 if (bufferIndex >= BUFFER_SIZE - 1) {
                                   sendMorseBuffer();
                                   break;
                                 }
                                 buffer[bufferIndex++] = (char)inByte;
                                 buffer[bufferIndex] = '\0'; // Null-terminate the string
                                 // Check for end of payload:
                                 if (i == payload.length() - 1) {
                                   sendMorseBuffer();
                                   break;
                                 }
                                 split_message_on_prosign((char)inByte, true);
                               }
                             Serial.write("\n");
                             tx_state = false;
                             publishTxState();
                           });
    }
}

void handleMQTT(void *handler_args, esp_event_base_t base, int32_t event_id, void *event_data) {
  auto *event = static_cast<esp_mqtt_event_handle_t>(event_data);
  mqttClient.onEventCallback(event);
}

/* // Interrupt function to call the Morse timer ISR */
/* void ARDUINO_ISR_ATTR onMorseTimer() { */
/*   Morse.timerISR(); */
/* } */

void clearBuffer() {
  memset(buffer, 0, BUFFER_SIZE); // Clear the buffer
  bufferIndex = 0;                // Reset the index
}

void publishMessage() {
  char topic[100];
  snprintf(topic, sizeof(topic), "%s/%s", MQTT_TOPIC_ROOT, "rx_message");
  mqttClient.publish(topic, buffer, MQTT_QOS, MQTT_RETAIN_MESSAGES);
}

void publishMessageBreak() {
  char topic[100];
  snprintf(topic, sizeof(topic), "%s/%s", MQTT_TOPIC_ROOT, "rx_message");
  mqttClient.publish(topic, " ", MQTT_QOS, MQTT_RETAIN_MESSAGES);
}

void publishRxStream(char ch) {
  if (MQTT_ENABLE_STREAMING) {
    char topic[100];
    snprintf(topic, sizeof(topic), "%s/%s", MQTT_TOPIC_ROOT, "rx_stream");
    char msg[2] = { ch, '\0' };
    mqttClient.publish(topic, msg, MQTT_QOS, MQTT_RETAIN_MESSAGES);
  }
}

void publishTxStream(char ch) {
  if (MQTT_ENABLE_STREAMING) {
    char topic[100];
    snprintf(topic, sizeof(topic), "%s/%s", MQTT_TOPIC_ROOT, "tx_stream");
    char msg[2] = { ch, '\0' };
    mqttClient.publish(topic, msg, MQTT_QOS, MQTT_RETAIN_MESSAGES);
  }
}

void publishTxState() {
  char topic[100];
  snprintf(topic, sizeof(topic), "%s/%s", MQTT_TOPIC_ROOT, "tx_state");
  mqttClient.publish("morse-bridge/tx_state", tx_state ? "1" : "0", MQTT_QOS, MQTT_RETAIN_MESSAGES);
  lastPublishStateTime = millis();
}

void sendMorseBuffer() {
  buffer[BUFFER_SIZE - 1] = '\0';
  for (int i = 0; i < BUFFER_SIZE && buffer[i] != '\0'; i++) {
    Morse.write(buffer[i]);
    lastTransmitTime = millis();
    Serial.write(buffer[i]);
    publishTxStream(buffer[i]);
  }
  Serial.write('\n');
  publishMessage();
  clearBuffer();
}

void split_message_on_prosign(char inByte, bool is_tx) {
  // Check if a space or end of word character is received
  if (inByte == ' ' || inByte == '\n' || inByte == '\r') {
    // Ignore the last character (space or newline)
    int checkIndex = bufferIndex - 1;
      
    // Check the last three characters in the buffer, considering possible leading spaces
    if (checkIndex >= 3) {
      if (strncmp(buffer + checkIndex - 3, " AR", 3) == 0 ||
          strncmp(buffer + checkIndex - 3, " BK", 3) == 0 ||
          strncmp(buffer + checkIndex - 3, " KN", 3) == 0 ||
          strncmp(buffer + checkIndex - 3, " SK", 3) == 0 ||
          strncmp(buffer + checkIndex - 3, " CL", 3) == 0 ||
          strncmp(buffer + checkIndex - 2, " K", 2) == 0) {
        if (is_tx) {
          sendMorseBuffer();
          Serial.write("\n"); //extra newline after sendMorseBuffer did too
          publishMessageBreak();
        } else {
          publishMessage();
          Serial.write("\n\n");
          publishMessageBreak();
          clearBuffer();
        }
      } else if (strncmp(buffer + checkIndex - 3, " BT", 3) == 0) {
        if (is_tx) {
          sendMorseBuffer();
        } else {
          publishMessage();
          Serial.write("\n");
          clearBuffer();
        }
      }
    } else if (checkIndex >= 2) {
      if (strncmp(buffer, "AR", 2) == 0 ||
          strncmp(buffer, "BK", 2) == 0 ||
          strncmp(buffer, "KN", 2) == 0 ||
          strncmp(buffer, "SK", 2) == 0 ||
          strncmp(buffer, "CL", 2) == 0 ||
          strncmp(buffer, " K", 2) == 0) {
        if (is_tx) {
          sendMorseBuffer();
          publishMessageBreak();
        } else {
          publishMessage();
          Serial.write("\n\n");
          publishMessageBreak();
          clearBuffer();
        }
      } else if (strncmp(buffer, "BT", 2) == 0) {
        if (is_tx) {
          sendMorseBuffer();
        } else {
          publishMessage();
          Serial.write("\n");
          clearBuffer();
        }
      }
    } else if (checkIndex >= 1) {
      if (strncmp(buffer, "K", 1) == 0) {
        if (is_tx) {
          sendMorseBuffer();
          publishMessageBreak();
        } else {
          publishMessage();
          Serial.write("\n\n");
          publishMessageBreak();
          clearBuffer();
        }
      }
    }
  }
}
