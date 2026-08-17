typedef struct {
    const char *statement_id;
    const char *source_path;
    unsigned int line;
    unsigned int col;
} SkDebugLocation;

static const SkDebugLocation sk_debug_locations[] = {
    /* SKADI_DEBUG_LOCATION_TABLE */
    {NULL, NULL, 0, 0}
};

#define SK_DEBUG_MAX_FRAMES 64
#define SK_DEBUG_MAX_LOCALS 64
#define SK_DEBUG_NAME_SIZE 96
#define SK_DEBUG_VALUE_SIZE 256

typedef struct {
    char name[SK_DEBUG_NAME_SIZE];
    char type_name[SK_DEBUG_NAME_SIZE];
    char value[SK_DEBUG_VALUE_SIZE];
} SkDebugLocal;

typedef struct {
    char function_name[SK_DEBUG_NAME_SIZE];
    char statement_id[SK_DEBUG_NAME_SIZE];
    SkDebugLocal locals[SK_DEBUG_MAX_LOCALS];
    size_t local_count;
} SkDebugFrame;

static SK_THREAD_LOCAL SkDebugFrame sk_debug_frames[SK_DEBUG_MAX_FRAMES];
static SK_THREAD_LOCAL size_t sk_debug_frame_count = 0;
static int sk_debug_initialized = 0;
static int sk_debug_step_mode = 0;
static const char *sk_debug_breakpoints = NULL;

#if defined(_WIN32)
typedef SOCKET SkDebugSocket;
#define SK_DEBUG_INVALID_SOCKET INVALID_SOCKET
static INIT_ONCE sk_debug_lock_once = INIT_ONCE_STATIC_INIT;
static CRITICAL_SECTION sk_debug_lock_value;

static BOOL CALLBACK sk_debug_init_lock(PINIT_ONCE once, PVOID parameter, PVOID *context) {
    (void)once;
    (void)parameter;
    (void)context;
    InitializeCriticalSection(&sk_debug_lock_value);
    return TRUE;
}

static void sk_debug_lock(void) {
    InitOnceExecuteOnce(&sk_debug_lock_once, sk_debug_init_lock, NULL, NULL);
    EnterCriticalSection(&sk_debug_lock_value);
}

static void sk_debug_unlock(void) {
    LeaveCriticalSection(&sk_debug_lock_value);
}

static unsigned long long sk_debug_thread_id(void) {
    return (unsigned long long)GetCurrentThreadId();
}

static void sk_debug_close_socket(SkDebugSocket socket_value) {
    closesocket(socket_value);
}
#else
typedef int SkDebugSocket;
#define SK_DEBUG_INVALID_SOCKET (-1)
static pthread_mutex_t sk_debug_lock_value = PTHREAD_MUTEX_INITIALIZER;

static void sk_debug_lock(void) {
    pthread_mutex_lock(&sk_debug_lock_value);
}

static void sk_debug_unlock(void) {
    pthread_mutex_unlock(&sk_debug_lock_value);
}

static unsigned long long sk_debug_thread_id(void) {
    return (unsigned long long)(uintptr_t)pthread_self();
}

static void sk_debug_close_socket(SkDebugSocket socket_value) {
    close(socket_value);
}
#endif

static SkDebugSocket sk_debug_socket = SK_DEBUG_INVALID_SOCKET;

static void sk_debug_copy(char *target, size_t capacity, const char *value) {
    if (capacity == 0) return;
    if (value == NULL) value = "<null>";
    snprintf(target, capacity, "%s", value);
}

static void sk_debug_enter(const char *function_name) {
    SkDebugFrame *frame;
    if (sk_debug_frame_count >= SK_DEBUG_MAX_FRAMES) return;
    frame = &sk_debug_frames[sk_debug_frame_count++];
    memset(frame, 0, sizeof(*frame));
    sk_debug_copy(frame->function_name, sizeof(frame->function_name), function_name);
}

static void sk_debug_leave(void) {
    if (sk_debug_frame_count > 0) sk_debug_frame_count--;
}

static SkDebugFrame *sk_debug_current_frame(void) {
    if (sk_debug_frame_count == 0) sk_debug_enter("<runtime>");
    return &sk_debug_frames[sk_debug_frame_count - 1];
}

static SkDebugLocal *sk_debug_find_local(SkDebugFrame *frame, const char *name) {
    size_t index;
    for (index = 0; index < frame->local_count; index++) {
        if (strcmp(frame->locals[index].name, name) == 0) return &frame->locals[index];
    }
    if (frame->local_count >= SK_DEBUG_MAX_LOCALS) return NULL;
    return &frame->locals[frame->local_count++];
}

static void sk_debug_set_local(const char *name, const char *type_name, const char *value) {
    SkDebugLocal *local = sk_debug_find_local(sk_debug_current_frame(), name);
    if (local == NULL) return;
    sk_debug_copy(local->name, sizeof(local->name), name);
    sk_debug_copy(local->type_name, sizeof(local->type_name), type_name);
    sk_debug_copy(local->value, sizeof(local->value), value);
}

static void sk_debug_local_i64(const char *name, const char *type_name, long long value) {
    char text[SK_DEBUG_VALUE_SIZE];
    snprintf(text, sizeof(text), "%lld", value);
    sk_debug_set_local(name, type_name, text);
}

static void sk_debug_local_f64(const char *name, const char *type_name, double value) {
    char text[SK_DEBUG_VALUE_SIZE];
    snprintf(text, sizeof(text), "%.17g", value);
    sk_debug_set_local(name, type_name, text);
}

static void sk_debug_local_bool(const char *name, const char *type_name, bool value) {
    sk_debug_set_local(name, type_name, value ? "true" : "false");
}

static void sk_debug_local_char(const char *name, const char *type_name, char value) {
    char text[SK_DEBUG_VALUE_SIZE];
    if (value >= 32 && value < 127) snprintf(text, sizeof(text), "'%c'", value);
    else snprintf(text, sizeof(text), "0x%02x", (unsigned char)value);
    sk_debug_set_local(name, type_name, text);
}

static void sk_debug_local_text(const char *name, const char *type_name, const char *value) {
    sk_debug_set_local(name, type_name, value);
}

static int sk_debug_has_breakpoint(const char *statement_id) {
    const char *cursor = sk_debug_breakpoints;
    size_t id_length = strlen(statement_id);
    if (cursor == NULL || *cursor == '\0') return 0;
    while (*cursor != '\0') {
        const char *end = strchr(cursor, ',');
        size_t length = end == NULL ? strlen(cursor) : (size_t)(end - cursor);
        if (length == id_length && strncmp(cursor, statement_id, length) == 0) return 1;
        if (end == NULL) break;
        cursor = end + 1;
    }
    return 0;
}

static const SkDebugLocation *sk_debug_find_location(const char *statement_id) {
    const SkDebugLocation *location = sk_debug_locations;
    while (location->statement_id != NULL) {
        if (strcmp(location->statement_id, statement_id) == 0) return location;
        location++;
    }
    return NULL;
}

static int sk_debug_send_all(const char *data, size_t length) {
    while (length > 0) {
        int sent = send(sk_debug_socket, data, (int)length, 0);
        if (sent <= 0) return 0;
        data += sent;
        length -= (size_t)sent;
    }
    return 1;
}

static int sk_debug_send_text(const char *text) {
    return sk_debug_send_all(text, strlen(text));
}

static int sk_debug_send_hex(const char *text) {
    static const char digits[] = "0123456789abcdef";
    char pair[2];
    const unsigned char *cursor = (const unsigned char *)(text == NULL ? "" : text);
    while (*cursor != 0) {
        pair[0] = digits[*cursor >> 4];
        pair[1] = digits[*cursor & 15];
        if (!sk_debug_send_all(pair, 2)) return 0;
        cursor++;
    }
    return 1;
}

static int sk_debug_open_transport(void) {
    const char *port_text = getenv("SKADI_DEBUG_PORT");
    long port;
    struct sockaddr_in address;
    if (port_text == NULL || *port_text == '\0') return 0;
    port = strtol(port_text, NULL, 10);
    if (port <= 0 || port > 65535) return 0;
#if defined(_WIN32)
    {
        WSADATA winsock;
        if (WSAStartup(MAKEWORD(2, 2), &winsock) != 0) return 0;
    }
#endif
    sk_debug_socket = socket(AF_INET, SOCK_STREAM, 0);
    if (sk_debug_socket == SK_DEBUG_INVALID_SOCKET) return 0;
    memset(&address, 0, sizeof(address));
    address.sin_family = AF_INET;
    address.sin_port = htons((unsigned short)port);
    address.sin_addr.s_addr = htonl(INADDR_LOOPBACK);
    if (connect(sk_debug_socket, (struct sockaddr *)&address, sizeof(address)) != 0) {
        sk_debug_close_socket(sk_debug_socket);
        sk_debug_socket = SK_DEBUG_INVALID_SOCKET;
        return 0;
    }
    return 1;
}

static int sk_debug_send_stop(const char *statement_id) {
    char number[64];
    size_t frame_offset;
    SkDebugFrame *current = sk_debug_current_frame();
    sk_debug_copy(current->statement_id, sizeof(current->statement_id), statement_id);
    if (!sk_debug_send_text("STOP\t")) return 0;
    snprintf(number, sizeof(number), "%llu", sk_debug_thread_id());
    if (!sk_debug_send_text(number) || !sk_debug_send_text("\t") ||
        !sk_debug_send_hex(statement_id) || !sk_debug_send_text("\n")) return 0;
    for (frame_offset = 0; frame_offset < sk_debug_frame_count; frame_offset++) {
        size_t frame_index = sk_debug_frame_count - frame_offset - 1;
        size_t local_index;
        SkDebugFrame *frame = &sk_debug_frames[frame_index];
        snprintf(number, sizeof(number), "%zu", frame_offset);
        if (!sk_debug_send_text("FRAME\t") || !sk_debug_send_text(number) ||
            !sk_debug_send_text("\t") || !sk_debug_send_hex(frame->function_name) ||
            !sk_debug_send_text("\t") || !sk_debug_send_hex(frame->statement_id) ||
            !sk_debug_send_text("\n")) return 0;
        for (local_index = 0; local_index < frame->local_count; local_index++) {
            SkDebugLocal *local = &frame->locals[local_index];
            if (!sk_debug_send_text("LOCAL\t") || !sk_debug_send_text(number) ||
                !sk_debug_send_text("\t") || !sk_debug_send_hex(local->name) ||
                !sk_debug_send_text("\t") || !sk_debug_send_hex(local->type_name) ||
                !sk_debug_send_text("\t") || !sk_debug_send_hex(local->value) ||
                !sk_debug_send_text("\n")) return 0;
        }
    }
    return sk_debug_send_text("END\n");
}

static int sk_debug_receive_command(char *command, size_t capacity) {
    size_t length = 0;
    while (length + 1 < capacity) {
        char byte;
        int received = recv(sk_debug_socket, &byte, 1, 0);
        if (received <= 0) return 0;
        if (byte == '\n') break;
        if (byte != '\r') command[length++] = byte;
    }
    command[length] = '\0';
    return 1;
}

static void sk_debug_terminal_stop(const char *statement_id) {
    char command[64];
    const SkDebugLocation *location = sk_debug_find_location(statement_id);
    if (location == NULL) fprintf(stderr, "\n[SKADI-DEBUG] stopped at %s\n", statement_id);
    else fprintf(stderr, "\n[SKADI-DEBUG] stopped at %s:%u:%u (%s)\n",
        location->source_path, location->line, location->col, statement_id);
    for (;;) {
        size_t length;
        fprintf(stderr, "(skadi-debug) ");
        fflush(stderr);
        if (fgets(command, sizeof(command), stdin) == NULL) {
            sk_debug_step_mode = 0;
            return;
        }
        length = strlen(command);
        while (length > 0 && (command[length - 1] == '\n' || command[length - 1] == '\r'))
            command[--length] = '\0';
        if (strcmp(command, "c") == 0 || strcmp(command, "continue") == 0) {
            sk_debug_step_mode = 0;
            return;
        }
        if (strcmp(command, "s") == 0 || strcmp(command, "step") == 0) {
            sk_debug_step_mode = 1;
            return;
        }
        if (strcmp(command, "q") == 0 || strcmp(command, "quit") == 0) exit(0);
        fprintf(stderr, "commands: continue (c), step (s), quit (q)\n");
    }
}

static void sk_debug_probe(const char *statement_id) {
    char command[64];
    sk_debug_lock();
    if (!sk_debug_initialized) {
        const char *step = getenv("SKADI_DEBUG_STEP");
        sk_debug_breakpoints = getenv("SKADI_DEBUG_BREAKPOINTS");
        sk_debug_step_mode = step != NULL && strcmp(step, "1") == 0;
        sk_debug_open_transport();
        sk_debug_initialized = 1;
    }
    if (!sk_debug_step_mode && !sk_debug_has_breakpoint(statement_id)) {
        sk_debug_unlock();
        return;
    }
    if (sk_debug_socket == SK_DEBUG_INVALID_SOCKET) {
        sk_debug_terminal_stop(statement_id);
        sk_debug_unlock();
        return;
    }
    if (!sk_debug_send_stop(statement_id) || !sk_debug_receive_command(command, sizeof(command))) {
        sk_debug_close_socket(sk_debug_socket);
        sk_debug_socket = SK_DEBUG_INVALID_SOCKET;
        sk_debug_step_mode = 0;
        sk_debug_unlock();
        return;
    }
    if (strcmp(command, "STEP") == 0) sk_debug_step_mode = 1;
    else if (strcmp(command, "QUIT") == 0) exit(0);
    else sk_debug_step_mode = 0;
    sk_debug_unlock();
}

