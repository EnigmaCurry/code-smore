// A GPIO morse code decoder with three OLED displays.

#include <SPI.h>
#include <Wire.h>
#include <Adafruit_GFX.h>
#include <Adafruit_SSD1306.h>
#include "Lewis.h"
#include <TimerOne.h>
#include <ctype.h> // toupper

#define WPM 20 // Default number of code words per minute
#define RX_PIN 2 // Receive morse code on GPIO pin RX_PIN
#define TX_PIN 3 // Send morse code on GPIO pin TX_PIN
#define SCREEN_WIDTH 128 // OLED display width, in pixels
#define SCREEN_HEIGHT 64 // OLED display height, in pixels
#define MORSE_INTERRUPT true // Run coroutine with morse transceiver running in the background
#define MORSE_INTERRUPT_FREQUENCY 100 // How often (Hz) to call the interrupt
#define OLED_RESET -1 // Reset pin # (or -1 if sharing Arduino reset pin)
#define SCREEN_ADDRESS 0x3C ///< See datasheet for Address; 0x3D for 128x64, 0x3C for 128x32
#define SERIAL_BAUD 9600
#define TIMEOUT 20 // Soft timeout in seconds of inactivity to clear the screen when new message arrives
#define HARD_TIMEOUT 180 // Hard timeout in seconds to clear screen for power saving purposes
#define BUFFER_SIZE 82 // a bit smaller than 90 because of some bug that overflows the screen
#define CONFIG_BUTTON 10 // Pin for the config menu
#define DEBOUNCE_DELAY 50 // Debounce delay in milliseconds

elapsedMillis currentTime;

// Have to put the displays on separate wire busses because they have the same unchangable address: 0x3c
Adafruit_SSD1306 displayRight(SCREEN_WIDTH, SCREEN_HEIGHT, &Wire, OLED_RESET);
Adafruit_SSD1306 displayMiddle(SCREEN_WIDTH, SCREEN_HEIGHT, &Wire1, OLED_RESET);
Adafruit_SSD1306 displayLeft(SCREEN_WIDTH, SCREEN_HEIGHT, &Wire2, OLED_RESET);

Lewis Morse;

char buffer[BUFFER_SIZE] = ""; // Buffer to store received characters
int bufferIndex = 0;           // Tracks the current position in the buffer
unsigned long lastReceivedTime = 0; // Tracks the last time a character was received
bool screenCleared = false; // Tracks if the screen has been cleared due to timeout
bool powerSaved = false; // Tracks if the screen has beeen cleared due to power saving

int wpm = WPM; // Current WPM setting, starts with the defined WPM
unsigned long lastConfigButtonPress = 0; // Tracks the last button press time
bool configButtonState = HIGH; // Current state of the button
bool lastConfigButtonState = HIGH; // Last known state of the button


void setup() {
  Serial.begin(SERIAL_BAUD);
  
  pinMode(CONFIG_BUTTON, INPUT_PULLUP);

  Morse.begin(RX_PIN, TX_PIN, wpm, MORSE_INTERRUPT);
  Timer1.initialize(MORSE_INTERRUPT_FREQUENCY * 100);
  Timer1.attachInterrupt(MorseISR);


  if (!displayLeft.begin(SSD1306_SWITCHCAPVCC, SCREEN_ADDRESS)) {
    Serial.println(F("ERROR: displayLeft failed"));
    for (;;)
      ; // Don't proceed, loop forever
  }  
  if (!displayMiddle.begin(SSD1306_SWITCHCAPVCC, SCREEN_ADDRESS)) {
    Serial.println(F("ERROR: displayMiddle failed"));
    for (;;)
      ; // Don't proceed, loop forever
  }
  if (!displayRight.begin(SSD1306_SWITCHCAPVCC, SCREEN_ADDRESS)) {
    Serial.println(F("ERROR: displayRight failed"));
    for (;;)
      ; // Don't proceed, loop forever
  }


  Serial.write("OK\n");
}

void loop() {
  handleConfigButtonPress();

  // Check if Morse data is available and store it in the buffer
  if (Morse.available()) {
    int inByte = toUpperCase(Morse.read());
    Serial.write(inByte);

    // Reset the timeout tracking
    lastReceivedTime = currentTime;
    screenCleared = false; // New data, ensure screen updates
    powerSaved = false;

    // Check for buffer overflow
    if (bufferIndex >= BUFFER_SIZE - 1) {
      clearBuffer(); // Clear the buffer if it overflows
    }

    // Append the new character to the buffer
    buffer[bufferIndex++] = (char)inByte;
    buffer[bufferIndex] = '\0'; // Null-terminate the string

    // Check if a space or end of word character is received
    if (inByte == ' ' || inByte == '\n' || inByte == '\r') {
      // Extract the last word from the buffer
      char lastWord[10] = ""; // Assuming Morse code words are shorter than 10 characters
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
            clearBuffer();       
            displayRight.clearDisplay(); 
            displayLeft.clearDisplay();
            displayMiddle.clearDisplay();
            screenCleared = true; 
            Serial.write("\n");
      }
    }
  }

  // Check if soft timeout has occurred
  if ((currentTime - lastReceivedTime > TIMEOUT * 1000) && !screenCleared) {
    clearBuffer();       // Clear the buffer
    displayRight.clearDisplay(); // Clear the screen
    displayLeft.clearDisplay();
    displayMiddle.clearDisplay();
    screenCleared = true; // Mark screen as cleared
    Serial.write("\n");
  }

  // Display the buffer on the OLED screen only if not cleared
  if (!screenCleared) {
    displayRight.clearDisplay();
    displayLeft.clearDisplay();
    displayMiddle.clearDisplay();
    drawtext(buffer);
    displayRight.display();
    displayMiddle.display();
    displayLeft.display();
  }

  // Send each serial byte to Morse output
  if (Serial.available()) {
    int inByte = Serial.read();
    Morse.write(inByte);
  }

  delay(10);
}

// Interrupt function to call the Morse timer ISR
void MorseISR() {
  // Check if hard timeout has occurred
  if ((currentTime - lastReceivedTime > HARD_TIMEOUT * 1000) && !powerSaved) {
    clearBuffer();
    displayRight.clearDisplay(); 
    displayLeft.clearDisplay();
    displayMiddle.clearDisplay();
    displayRight.display();
    displayLeft.display();
    displayMiddle.display();
    screenCleared = true;
    powerSaved = true;
    Serial.write("\n\n");
  }

  Morse.timerISR();
}

void handleConfigButtonPress() {
  // Check for button press
  bool currentButtonState = digitalRead(CONFIG_BUTTON);
  if (currentButtonState == LOW && lastConfigButtonState == HIGH &&
      (currentTime - lastConfigButtonPress > DEBOUNCE_DELAY)) {
    // Button was pressed
    lastConfigButtonPress = currentTime;
    wpm = (wpm < 30) ? wpm + 5 : 10; // Increment WPM and wrap around
    Morse.begin(RX_PIN, TX_PIN, wpm, MORSE_INTERRUPT);

    // Display the WPM on the screens
    displayRight.clearDisplay();
    displayMiddle.clearDisplay();
    displayLeft.clearDisplay();

    char wpmMessage[16];
    sprintf(wpmMessage, "WPM: %d", wpm);

    displayRight.setTextSize(2);
    displayRight.setTextColor(SSD1306_WHITE);
    displayRight.setCursor(0, 0);
    displayRight.println(wpmMessage);

    displayMiddle.setTextSize(2);
    displayMiddle.setTextColor(SSD1306_WHITE);
    displayMiddle.setCursor(0, 0);
    displayMiddle.println(wpmMessage);

    displayLeft.setTextSize(2);
    displayLeft.setTextColor(SSD1306_WHITE);
    displayLeft.setCursor(0, 0);
    displayLeft.println(wpmMessage);

    displayRight.display();
    displayMiddle.display();
    displayLeft.display();

    Serial.print("# WPM: ");
    Serial.println(wpm);

    delay(2000); // Show the message for 2 seconds

    displayRight.clearDisplay();
    displayMiddle.clearDisplay();
    displayLeft.clearDisplay();
    displayRight.display();
    displayMiddle.display();
    displayLeft.display();
  }
  lastConfigButtonState = currentButtonState;
}

// Function to clear the buffer
void clearBuffer() {
  memset(buffer, 0, BUFFER_SIZE); // Clear the buffer
  bufferIndex = 0;                // Reset the index
}

void drawtext(const char text[]) {
  const int maxCharsPerLine = 10; // Characters per display per line
  const int linesPerDisplay = 3; // Number of lines per display
  char lineBuffer[maxCharsPerLine + 1] = ""; // Buffer for each line of text
  int currentChar = 0; // Index in the main text buffer
  int textLength = strlen(text);

  for (int line = 0; line < linesPerDisplay; line++) {
    // Left display
    currentChar = getNextSegment(text, currentChar, textLength, maxCharsPerLine, lineBuffer);
    displayLeft.setTextSize(2);
    displayLeft.setTextColor(SSD1306_WHITE);
    displayLeft.setCursor(0, line * 10 * 2);
    displayLeft.println(lineBuffer);

    // Middle display
    currentChar = getNextSegment(text, currentChar, textLength, maxCharsPerLine, lineBuffer);
    displayMiddle.setTextSize(2);
    displayMiddle.setTextColor(SSD1306_WHITE);
    displayMiddle.setCursor(0, line * 10 * 2);
    displayMiddle.println(lineBuffer);

    // Right display
    currentChar = getNextSegment(text, currentChar, textLength, maxCharsPerLine, lineBuffer);
    displayRight.setTextSize(2);
    displayRight.setTextColor(SSD1306_WHITE);
    displayRight.setCursor(0, line * 10 * 2);
    displayRight.println(lineBuffer);
  }
}

// Helper function to convert a string to uppercase
void toUpperCase(char *text) {
  while (*text) {
    *text = toupper(*text);
    text++;
  }
}

int getNextSegment(const char *text, int start, int textLength, int maxChars, char *buffer) {
  int end = start;

  // Check if we are at the end of the text
  if (start >= textLength) {
    buffer[0] = '\0'; // Empty segment
    return textLength;
  }

  // Attempt to fit a segment of maxChars
  while (end < textLength && end - start < maxChars) {
    if (text[end] == ' ') {
      end++; // Include spaces in the segment
    } else {
      // Peek ahead for the next word
      int wordEnd = end;
      while (wordEnd < textLength && text[wordEnd] != ' ') {
        wordEnd++;
      }

      // If the word fits, include it
      if (wordEnd - start <= maxChars) {
        end = wordEnd; // Include the entire word
      } else if (wordEnd - end > maxChars) {
        // If the word is longer than maxChars, include as much as fits
        end = start + maxChars;
        break;
      } else {
        // Stop before starting this word
        break;
      }
    }
  }

  // Copy the segment into the buffer
  int segmentLength = end - start;
  strncpy(buffer, text + start, segmentLength);
  buffer[segmentLength] = '\0'; // Null-terminate the buffer

  // Skip any trailing whitespace for the next segment
  while (end < textLength && text[end] == ' ') {
    end++;
  }

  // Return the next starting index
  return end;
}



// Helper function to find the first non-whitespace character in a range
int findNonWhitespace(const char *text, int start, int end) {
  for (int i = start; i < end; i++) {
    if (text[i] != ' ' && text[i] != '\t' && text[i] != '\n' && text[i] != '\r') {
      return i; // Return the index of the first non-whitespace character
    }
  }
  return start; // If no non-whitespace is found, return the start index
}
