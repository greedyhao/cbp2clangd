#include <stdio.h>

void chatbot_init() {
    printf("Chatbot initialized\n");
}

void chatbot_process(const char* input, char* output) {
    snprintf(output, 100, "You said: %s", input);
}