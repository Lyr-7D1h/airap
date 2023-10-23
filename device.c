#include <stdio.h>
#include <pulse/pulseaudio.h>
#include <pulse/mainloop.h>

pa_mainloop* mainloop;
pa_context* context;

// Initialize main loop
mainloop = pa_mainloop_new();
context = pa_context_new(pa_mainloop_get_api(mainloop), "YourAppName");


void context_state_callback(pa_context* c, void* userdata) {
    pa_operation* operation;

    if (pa_context_get_state(c) == PA_CONTEXT_READY) {
        operation = pa_context_get_sink_input_info_list(c, sink_input_info_callback, NULL);
        pa_operation_unref(operation);
    }
}

void sink_input_info_callback(pa_context* c, const pa_sink_input_info* i, int eol, void* userdata) {
    if (!eol && i) {
        // Access the stream name from the sink input information
        const char* stream_name = pa_proplist_gets(i->proplist, "application.name");
        if (stream_name) {
            printf("Stream Name: %s\n", stream_name);
        }
    } else if (eol < 0) {
        // Handle error
    } else {
        // All sink input info has been processed
        pa_context_disconnect(c);
    }
}

int main() {
    // Initialize the context, set up callbacks, and connect to the PulseAudio server
    // ...

    // Run the main loop
    pa_mainloop_run(mainloop, NULL);

    // Cleanup and exit
    pa_context_unref(context);
    pa_mainloop_free(mainloop);

    return 0;
}
        pa_operation_unref(operation);
