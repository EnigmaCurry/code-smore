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
const bool MORSE_INTERRUPT = true;
const int MORSE_INTERRUPT_FREQUENCY = 100;
const int MORSE_RECEIVE_PIN = 4;
const int MORSE_SEND_PIN = 2;
const int MORSE_MESSAGE_TIMEOUT = 15; // Seconds after which an incomplete message will finalize
const int BUFFER_SIZE = 500;
const bool MQTT_RETAIN_MESSAGES = false;

ESP32MQTTClient mqttClient;
Lewis Morse;
unsigned long lastReceivedTime = 0; // Tracks the last time a character was received
char buffer[BUFFER_SIZE] = ""; // Buffer the current message
unsigned int bufferIndex = 0;           // Tracks the current position in the buffer

void setup()
{
  Serial.begin(115200);
  /* log_i(); */
  /* log_i("setup, ESP.getSdkVersion(): "); */
  /* log_i("%s", ESP.getSdkVersion()); */

  connectToWifi();
    mqttClient.enableDebuggingMessages();

  mqttClient.setURI(SECRET_MQTT_SERVER);
  mqttClient.enableLastWillMessage("lwt", "I am going offline");
  mqttClient.setKeepAlive(30);
  mqttClient.setClientCert(TLS_CERT);
  mqttClient.setKey(TLS_KEY);
  mqttClient.setCaCert(TLS_CA_CERT);
  mqttClient.loopStart();

  pinMode(MORSE_RECEIVE_PIN, INPUT);
  Morse.begin(MORSE_RECEIVE_PIN, MORSE_SEND_PIN, WPM, MORSE_INTERRUPT);

  Serial.write("OK\n");
}

void loop() {
  Morse.checkIncoming();
  if (Morse.available()) {
    int inByte = toUpperCase(Morse.read());
    Serial.write(inByte);
    lastReceivedTime = millis();

    // Check for buffer overflow
    if (bufferIndex >= BUFFER_SIZE - 1) {
      publishMessage();
      clearBuffer();
    }
    
    buffer[bufferIndex++] = (char)inByte;
    buffer[bufferIndex] = '\0'; // Null-terminate the string
    
    // Check if a space or end of word character is received
    if (inByte == ' ' || inByte == '\n' || inByte == '\r') {
      // Extract the last word from the buffer
      char lastWord[20] = ""; // Assuming Morse code words are shorter than 20 characters
      int lastWordStart = bufferIndex - 2;
      while (lastWordStart >= 0 && buffer[lastWordStart] != ' ') {
        lastWordStart--;
      }
      lastWordStart++; // Move to the first character of the word

      strncpy(lastWord, buffer + lastWordStart, bufferIndex - lastWordStart - 1);
      lastWord[bufferIndex - lastWordStart - 1] = '\0'; // Null-terminate the word

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
      }
    }
  }

  // Check if an incomplete message has been idle and finalize the message log.
  if (bufferIndex > 0 && (millis() - lastReceivedTime > MORSE_MESSAGE_TIMEOUT * 1000)) {
    publishMessage();
    clearBuffer();
    Serial.write("\n");
  }

  // Send each serial byte to Morse output
  if (Serial.available()) {
    int inByte = Serial.read();
    Morse.write(inByte);
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


void onMqttConnect(esp_mqtt_client_handle_t client)
{
  if (mqttClient.isMyTurn(client)) // can be omitted if only one client
  {
    mqttClient.subscribe(MQTT_TOPIC_ROOT, [](const String &payload)
                         {
                           //log_i("%s: %s", subscribeTopic, payload.c_str()); 
                         });
  }
}

void handleMQTT(void *handler_args, esp_event_base_t base, int32_t event_id, void *event_data) {
  auto *event = static_cast<esp_mqtt_event_handle_t>(event_data);
  mqttClient.onEventCallback(event);
}

// Interrupt function to call the Morse timer ISR
void MorseISR() {
  Morse.timerISR();
}

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
  char message_topic[100];
  snprintf(message_topic, sizeof(message_topic), "%s/%s", MQTT_TOPIC_ROOT, "messages");
  mqttClient.publish(message_topic, buffer, 2, MQTT_RETAIN_MESSAGES);
}
