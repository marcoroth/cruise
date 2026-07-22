#include <ruby.h>
#include <ruby/encoding.h>
#include <ruby/thread.h>
#include <string.h>
#include "include/cruise.h"

#define CRUISE_WAIT_TIMEOUT_MS 200

static VALUE make_utf8_string(const char *cstring) {
  return rb_enc_str_new_cstr(cstring, rb_utf8_encoding());
}

static void fill_c_strings(VALUE array, const char **buffer) {
  long length = RARRAY_LEN(array);

  for (long i = 0; i < length; i++) {
    VALUE string = rb_ary_entry(array, i);

    buffer[i] = StringValueCStr(string);
  }
}

struct next_event_args {
  struct Watcher *watcher;
  CruiseEvent *event;
  CruiseWaitStatus status;
};

static void *next_event_without_gvl(void *data) {
  struct next_event_args *args = data;

  args->status = cruise_watcher_next_event(args->watcher, CRUISE_WAIT_TIMEOUT_MS, args->event);

  return NULL;
}

static void unblock_wait(void *data) {
  (void) data;
}

struct watch_context {
  struct Watcher *watcher;
  VALUE callback;
};

static VALUE watch_loop(VALUE arg) {
  struct watch_context *context = (struct watch_context *) arg;

  while (1) {
    CruiseEvent event = { NULL, NULL };
    struct next_event_args args = { context->watcher, &event, CRUISE_WAIT_STATUS_TIMEOUT };

    rb_thread_call_without_gvl(next_event_without_gvl, &args, unblock_wait, NULL);

    switch (args.status) {
      case CRUISE_WAIT_STATUS_EVENT: {
        VALUE path = make_utf8_string(event.path);
        VALUE kind = make_utf8_string(event.kind);

        cruise_string_free(event.path);
        cruise_string_free(event.kind);

        VALUE cruise_event = rb_funcall(rb_path2class("Cruise::Event"), rb_intern("new"), 2, path, kind);
        rb_funcall(context->callback, rb_intern("call"), 1, cruise_event);

        break;
      }

      case CRUISE_WAIT_STATUS_TIMEOUT:
        rb_thread_check_ints();

        break;

      case CRUISE_WAIT_STATUS_DISCONNECTED:
        rb_raise(rb_eRuntimeError, "Watcher channel disconnected unexpectedly");
    }
  }

  return Qnil;
}

static VALUE watch_ensure(VALUE arg) {
  struct watch_context *context = (struct watch_context *) arg;

  if (context->watcher) {
    cruise_watcher_free(context->watcher);
    context->watcher = NULL;
  }

  return Qnil;
}

/* Cruise._watch(paths, callback, debounce, globs, only_kinds) */
static VALUE cruise_watch(VALUE self, VALUE paths, VALUE callback, VALUE debounce, VALUE globs, VALUE only_kinds) {
  (void) self;

  long path_count = RARRAY_LEN(paths);
  long glob_count = RARRAY_LEN(globs);
  long only_count = RARRAY_LEN(only_kinds);

  const char **path_ptrs = path_count > 0 ? ALLOCA_N(const char *, path_count) : NULL;
  const char **glob_ptrs = glob_count > 0 ? ALLOCA_N(const char *, glob_count) : NULL;
  const char **only_ptrs = only_count > 0 ? ALLOCA_N(const char *, only_count) : NULL;

  if (path_ptrs) fill_c_strings(paths, path_ptrs);
  if (glob_ptrs) fill_c_strings(globs, glob_ptrs);
  if (only_ptrs) fill_c_strings(only_kinds, only_ptrs);

  char *error = NULL;
  struct Watcher *watcher = cruise_watcher_new(
    path_ptrs, (size_t) path_count,
    NUM2DBL(debounce),
    glob_ptrs, (size_t) glob_count,
    only_ptrs, (size_t) only_count,
    &error
  );

  if (!watcher) {
    VALUE message = error ? make_utf8_string(error) : rb_str_new_cstr("Failed to create watcher");

    if (error) cruise_string_free(error);

    const char *message_cstr = StringValueCStr(message);
    VALUE error_class = rb_eRuntimeError;

    if (strstr(message_cstr, "does not exist") || strstr(message_cstr, "Invalid glob pattern")) {
      error_class = rb_eArgError;
    }

    rb_raise(error_class, "%s", message_cstr);
  }

  struct watch_context context = { watcher, callback };

  return rb_ensure(watch_loop, (VALUE) &context, watch_ensure, (VALUE) &context);
}

void Init_cruise(void) {
  VALUE rb_mCruise = rb_define_module("Cruise");

  rb_define_module_function(rb_mCruise, "_watch", cruise_watch, 5);
}
