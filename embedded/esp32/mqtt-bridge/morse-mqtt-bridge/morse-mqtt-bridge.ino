// ESP32 Morse Code MQTT transceiver  

// Set your WiFi credentials and MQTT settings in secrets.h
// This controller will publish to the following MQTT topics based on your root topic name:
///  {MQTT_TOPIC_ROOT}/rx_stream    - the stream of morse code letters received
///  {MQTT_TOPIC_ROOT}/rx_messages  - complete messages received split by prosign or timeout
//  {MQTT_TOPIC_ROOT}/tx_state  - streams the state of the tramsmit mode (1=tx 0=rx)

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
  if (!is_transmitting() && Morse.available()) {
    int inByte = toUpperCase(Morse.read());
    Serial.write(inByte);
    lastReceivedTime = millis();

    publishStream(inByte);
    
    // Check for buffer overflow
    if (bufferIndex >= BUFFER_SIZE - 1) {
      publishMessage();
      clearBuffer();
    }
    
    buffer[bufferIndex++] = (char)inByte;
    buffer[bufferIndex] = '\0'; // Null-terminate the string
    
    // Check if a space or end of word character is received
    if (inByte == ' ' || inByte == '\n' || inByte == '\r') {
      // Extract the last word from the buffer. If its longer than 5 characters truncate.
      char lastWord[6] = "";
      int lastWordStart = bufferIndex - 2;

      while (lastWordStart >= 0 && buffer[lastWordStart] != ' ') {
        lastWordStart--;
      }
      lastWordStart++; // Move to the first character of the word

      int wordLength = bufferIndex - lastWordStart - 1;
      if (wordLength > 5) {
        wordLength = 5; // Truncate to fit lastWord
      }

      strncpy(lastWord, buffer + lastWordStart, wordLength);
      lastWord[wordLength] = '\0'; // Ensure null-termination

      // Convert the last word to uppercase (in case it's not already)
      toUpperCase(lastWord);

      // Check if the last word matches any prosigns
      if (strcmp(lastWord, "AR") == 0 || strcmp(lastWord, "BK") == 0 || 
          strcmp(lastWord, "K") == 0 || strcmp(lastWord, "KN") == 0 || 
          strcmp(lastWord, "SK") == 0 || strcmp(lastWord, "CL") == 0 || 
          strcmp(lastWord, "BT") == 0) {
        publishMessage();
        clearBuffer();       
        Serial.write("\n");
        if (strcmp(lastWord, "BT") != 0) {
          publishMessageBreak();
          Serial.write("\n");
        }
      }
    }
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
    int inByte = Serial.read();
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
    snprintf(topic, sizeof(topic), "%s/%s", MQTT_TOPIC_ROOT, "tx_messages");
    mqttClient.subscribe(topic, [](const String &payload)
                         {
                           tx_state = true;
                           publishTxState();
                           for (size_t i = 0; i < payload.length(); i++)
                             {
                               char inByte = payload.charAt(i);
                               Morse.write(inByte);
                               lastTransmitTime = millis();
                             }
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

void toUpperCase(char *text) {
  while (*text) {
    *text = toupper(*text);
    text++;
  }
}

void publishMessage() {
  char topic[100];
  snprintf(topic, sizeof(topic), "%s/%s", MQTT_TOPIC_ROOT, "rx_messages");
  mqttClient.publish(topic, buffer, 2, MQTT_RETAIN_MESSAGES);
}

void publishMessageBreak() {
  char topic[100];
  snprintf(topic, sizeof(topic), "%s/%s", MQTT_TOPIC_ROOT, "rx_messages");
  mqttClient.publish(topic, " ", 2, MQTT_RETAIN_MESSAGES);
}

void publishStream(char ch) {
    char topic[100];
    snprintf(topic, sizeof(topic), "%s/%s", MQTT_TOPIC_ROOT, "rx_stream");
    char msg[2] = { ch, '\0' };
    mqttClient.publish(topic, msg, 1, MQTT_RETAIN_MESSAGES);
}

void publishTxState() {
    char topic[100];
    snprintf(topic, sizeof(topic), "%s/%s", MQTT_TOPIC_ROOT, "tx_state");
    mqttClient.publish("morse-bridge/tx_state", tx_state ? "1" : "0", 0, MQTT_RETAIN_MESSAGES);
    lastPublishStateTime = millis();
}
