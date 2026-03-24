<div align="center">
  <h1>Cruise</h1>
  <h4>A fast, native file watcher for Ruby.</h4>

  <p>
    <a href="https://rubygems.org/gems/cruise"><img alt="Gem Version" src="https://img.shields.io/gem/v/cruise"></a>
    <a href="https://github.com/marcoroth/cruise/blob/main/LICENSE.txt"><img alt="License" src="https://img.shields.io/github/license/marcoroth/cruise"></a>
  </p>

  <p>Rust-powered file system watcher with native OS integration.<br/>Uses FSEvents on macOS and inotify on Linux.</p>
</div>

## Installation

**Add to your Gemfile:**

```ruby
gem "cruise"
```

**Or install directly:**

```bash
gem install cruise
```

Precompiled native gems are available for macOS and Linux. If a precompiled gem isn't available for your platform, it will compile from source (requires Rust).

## Usage

### Basic Example

Watch a directory for changes:

```ruby
require "cruise"

Cruise.watch("app/views") do |event|
  puts "#{event.kind}: #{event.path}"
end
```

The callback receives a `Cruise::Event` with two attributes:

| Attribute | Description |
|-----------|-------------|
| `event.path` | Absolute path to the changed file |
| `event.kind` | One of: `"created"`, `"modified"`, `"renamed"`, `"removed"`, `"accessed"`, `"changed"` |

### Multiple Directories

Watch several paths at once:

```ruby
Cruise.watch("app/views", "app/components", "lib/templates") do |event|
  puts event.inspect
  # => #<Cruise::Event kind="modified" path="/app/views/users/show.html.erb">
end
```

### Ruby Threads

Cruise releases the GVL while waiting for filesystem events, so Ruby threads run freely:

```ruby
Thread.new { do_background_work }

Cruise.watch("src") do |event|
  puts event
end
```

### Glob Filtering

Only receive events for files matching a pattern:

```ruby
Cruise.watch("app/views", glob: "**/*.html.erb") do |event|
  puts event
end
```

Multiple patterns:

```ruby
Cruise.watch("app", glob: ["**/*.html.erb", "**/*.html"]) do |event|
  puts event
end
```

### Debounce

Configure the debounce interval (default: 100ms):

```ruby
Cruise.watch("src", debounce: 0.5) do |event|
  puts event
end
```

### Proc Callback

You can also pass a `Proc` via the `callback:` keyword:

```ruby
handler = proc { |event| puts event }

Cruise.watch("app/views", callback: handler)
```

### Stopping

The watcher runs in a blocking loop. Use `Interrupt` to stop it cleanly:

```ruby
begin
  Cruise.watch("app") do |event|
    # process event
  end
rescue Interrupt
  puts "Stopped."
end
```

## How It Works

Cruise is a Ruby binding (via [Magnus](https://github.com/matsadler/magnus) and [rb-sys](https://github.com/oxidize-rb/rb-sys)) around the Rust [notify](https://github.com/notify-rs/notify) crate.

1. `Cruise.watch` sets up a [notify](https://github.com/notify-rs/notify) watcher with event debouncing (100ms)
2. A background thread monitors filesystem events using the OS-native API
3. The main loop calls `rb_thread_call_without_gvl` to wait without blocking Ruby
4. When an event arrives, the GVL is re-acquired and your callback is invoked

### Platform Backends

| Platform | Backend | API |
|----------|---------|-----|
| macOS | FSEvents | `CoreServices` framework |
| Linux | inotify | `inotify_init1` syscall |

All backends watch recursively by default.

## Development

**Requirements:** Rust toolchain, Ruby 3.2+

```bash
git clone https://github.com/marcoroth/cruise
cd cruise
bundle install
bundle exec rake compile
bundle exec rake test
```

### Cross-compilation

Cruise uses [rake-compiler](https://github.com/rake-compiler/rake-compiler) and [rake-compiler-dock](https://github.com/rake-compiler/rake-compiler-dock) for building native gems:

```bash
bundle exec rake gem:native
```

## License

MIT License. See [LICENSE.txt](LICENSE.txt).
