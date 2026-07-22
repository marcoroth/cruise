#include <ruby.h>
#include <ruby/encoding.h>
#include <string.h>
#include "include/cruise.h"

static VALUE rb_mCruise;
static VALUE rb_cWatcher;
static VALUE rb_cEvent;

static VALUE make_utf8_string(const char *cstring) {
  return rb_enc_str_new_cstr(cstring, rb_utf8_encoding());
}

static void fill_c_strings(VALUE array, const char **buffer) {
  long length = RARRAY_LEN(array);

  for (long index = 0; index < length; index++) {
    VALUE string = rb_ary_entry(array, index);

    buffer[index] = StringValueCStr(string);
  }
}

static void watcher_free(void *pointer) {
  if (pointer) {
    cruise_watcher_free((struct Watcher *) pointer);
  }
}

static size_t watcher_size(const void *pointer) {
  (void) pointer;

  return sizeof(void *);
}

static const rb_data_type_t watcher_type = {
  .wrap_struct_name = "Cruise::Watcher",
  .function = {
    .dfree = watcher_free,
    .dsize = watcher_size,
  },
  .flags = RUBY_TYPED_FREE_IMMEDIATELY,
};

static VALUE watcher_alloc(VALUE klass) {
  return TypedData_Wrap_Struct(klass, &watcher_type, NULL);
}

static struct Watcher *get_watcher(VALUE self) {
  struct Watcher *watcher;

  TypedData_Get_Struct(self, struct Watcher, &watcher_type, watcher);

  return watcher;
}

static VALUE watcher_initialize_native(VALUE self, VALUE paths, VALUE debounce, VALUE globs, VALUE only_kinds) {
  long path_count = RARRAY_LEN(paths);
  long glob_count = RARRAY_LEN(globs);
  long only_count = RARRAY_LEN(only_kinds);

  const char **path_pointers = path_count > 0 ? ALLOCA_N(const char *, path_count) : NULL;
  const char **glob_pointers = glob_count > 0 ? ALLOCA_N(const char *, glob_count) : NULL;
  const char **only_pointers = only_count > 0 ? ALLOCA_N(const char *, only_count) : NULL;

  if (path_pointers) fill_c_strings(paths, path_pointers);
  if (glob_pointers) fill_c_strings(globs, glob_pointers);
  if (only_pointers) fill_c_strings(only_kinds, only_pointers);

  int read_fd = -1;
  char *error = NULL;

  struct Watcher *watcher = cruise_watcher_new(
    path_pointers, (size_t) path_count,
    NUM2DBL(debounce),
    glob_pointers, (size_t) glob_count,
    only_pointers, (size_t) only_count,
    &read_fd,
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

  RTYPEDDATA_DATA(self) = watcher;

  VALUE io = rb_funcall(rb_cIO, rb_intern("for_fd"), 2, INT2NUM(read_fd), rb_str_new_cstr("r"));
  rb_iv_set(self, "@io", io);

  return self;
}

// watcher.io
static VALUE watcher_io(VALUE self) {
  return rb_iv_get(self, "@io");
}

// watcher.poll
static VALUE watcher_poll(VALUE self) {
  struct Watcher *watcher = get_watcher(self);

  if (!watcher) return Qnil;

  CruiseEvent event = { NULL, NULL };

  if (!cruise_watcher_poll(watcher, &event)) return Qnil;

  VALUE path = make_utf8_string(event.path);
  VALUE kind = make_utf8_string(event.kind);

  cruise_string_free(event.path);
  cruise_string_free(event.kind);

  return rb_funcall(rb_cEvent, rb_intern("new"), 2, path, kind);
}

// watcher.close
static VALUE watcher_close(VALUE self) {
  struct Watcher *watcher = get_watcher(self);

  if (watcher) {
    cruise_watcher_free(watcher);
    RTYPEDDATA_DATA(self) = NULL;
  }

  VALUE io = rb_iv_get(self, "@io");

  if (!NIL_P(io) && !RTEST(rb_funcall(io, rb_intern("closed?"), 0))) {
    rb_funcall(io, rb_intern("close"), 0);
  }

  return Qnil;
}

void Init_cruise(void) {
  rb_mCruise = rb_define_module("Cruise");
  rb_cEvent = rb_const_get(rb_mCruise, rb_intern("Event"));

  rb_cWatcher = rb_define_class_under(rb_mCruise, "Watcher", rb_cObject);

  rb_define_alloc_func(rb_cWatcher, watcher_alloc);
  rb_define_private_method(rb_cWatcher, "initialize_native", watcher_initialize_native, 4);
  rb_define_method(rb_cWatcher, "io", watcher_io, 0);
  rb_define_method(rb_cWatcher, "poll", watcher_poll, 0);
  rb_define_method(rb_cWatcher, "close", watcher_close, 0);
}
