// ASCII fire animation - Zig port of https://github.com/Wyatt915/fireplace
// Original by Wyatt Sheffield, MIT License
// Uses cellular automata with Wolfram Rule 60 for heat source flickering
// Dynamic "breathing" effect from issue #15 by lexaterra

const std = @import("std");
const posix = std.posix;

// Signal handling for resize
var sig_resized: std.atomic.Value(bool) = std.atomic.Value(bool).init(false);

fn sigwinchHandler(_: c_int) callconv(.c) void {
    sig_resized.store(true, .release);
}

// xterm-256 color palette from fireplace
const PALETTE = [_]u8{ 233, 52, 88, 124, 160, 166, 202, 208, 214, 220, 226, 227, 228, 229, 230, 231 };
const PALETTE_SZ: usize = PALETTE.len - 1;

// Edge type for fire shape detection
const EdgeType = enum { internal, top, bottom, left, right, tl, tr, bl, br };

// Character palette for fire rendering (all fields are UTF-8 strings for random selection)
const CharPalette = struct {
    internal: []const u8,
    edge_top: []const u8,
    edge_left: []const u8,
    edge_right: []const u8,
    edge_bottom: []const u8,
    corner_tl: []const u8,
    corner_tr: []const u8,
    corner_bl: []const u8,
    corner_br: []const u8,
};

const PALETTES = struct {
    // Classic solid blocks (default)
    const blocks = CharPalette{
        .internal = "█▓▒░",
        .edge_top = "▀^'",
        .edge_bottom = "▄._",
        .edge_left = "▌<(",
        .edge_right = "▐>)",
        .corner_tl = "▛/",
        .corner_tr = "▜\\",
        .corner_bl = "▙\\",
        .corner_br = "▟/",
    };

    // ASCII-only - very distinct from blocks
    const ascii = CharPalette{
        .internal = "#$%&*@",
        .edge_top = "^'`~",
        .edge_bottom = "v._,",
        .edge_left = "<([{",
        .edge_right = ">)]}",
        .corner_tl = "/7",
        .corner_tr = "\\",
        .corner_bl = "\\",
        .corner_br = "/",
    };

    // Dots/particles
    const dots = CharPalette{
        .internal = "●◉◎○·",
        .edge_top = "˙'`",
        .edge_bottom = ".,_",
        .edge_left = "‹<(",
        .edge_right = "›>)",
        .corner_tl = "╭/",
        .corner_tr = "╮\\",
        .corner_bl = "╰\\",
        .corner_br = "╯/",
    };

    // Hash marks - very visible
    const hash = CharPalette{
        .internal = "#@%&$*",
        .edge_top = "~^'`",
        .edge_bottom = "_.-,",
        .edge_left = "[<({",
        .edge_right = "]>)}",
        .corner_tl = "+/\\",
        .corner_tr = "+/\\",
        .corner_bl = "+/\\",
        .corner_br = "+/\\",
    };
};

var current_palette: *const CharPalette = &PALETTES.blocks;

var prng: std.Random.DefaultPrng = undefined;
var random: std.Random = undefined;

fn rand_int(max: usize) usize {
    return random.intRangeLessThan(usize, 0, max);
}

fn rand_bool() bool {
    return random.boolean();
}

// Wolfram's Elementary Cellular Automaton Rule 60
fn wolfram(world: []u8, rule: u8) void {
    const width = world.len;
    var next: [4096]u8 = undefined;

    for (0..width) |i| {
        const l: u3 = @intCast(world[if (i > 0) i - 1 else width - 1]);
        const c: u3 = @intCast(world[i]);
        const r: u3 = @intCast(world[(i + 1) % width]);
        const current: u3 = (l << 2) | (c << 1) | r;
        next[i] = (rule >> current) & 1;
    }

    @memcpy(world, next[0..width]);
}

// As a cell cools it has a higher chance of cooling again
fn cooldown(heat: i32) i32 {
    if (heat == 0) return 0;
    var h = heat;
    const r: i32 = @intCast(rand_int(@intCast(heat)));
    if (r == 0) h -= 1;
    return h;
}

const Fire = struct {
    field: []i32,
    count: []i32,
    heater: []u8,
    hotplate: []u8,
    cols: usize,
    rows: usize,
    height_record: usize,
    max_temp: i32,
    shuffle: u32,
    bounce: i32,
    rand_height: i32,
    allocator: std.mem.Allocator,

    const min_temp: i32 = 8;
    const max_temp_cap: i32 = 14;
    const wolfram_rule: u8 = 60;

    fn init(allocator: std.mem.Allocator, cols: usize, rows: usize) !Fire {
        const field = try allocator.alloc(i32, rows * cols);
        const count = try allocator.alloc(i32, rows * cols);
        const heater = try allocator.alloc(u8, cols);
        const hotplate = try allocator.alloc(u8, cols);

        @memset(field, 0);
        @memset(count, 0);
        @memset(hotplate, 0);

        for (heater) |*h| {
            h.* = if (rand_bool()) 1 else 0;
        }

        return Fire{
            .field = field,
            .count = count,
            .heater = heater,
            .hotplate = hotplate,
            .cols = cols,
            .rows = rows,
            .height_record = rows,
            .max_temp = 10,
            .shuffle = 0,
            .bounce = 10,
            .rand_height = 0,
            .allocator = allocator,
        };
    }

    fn deinit(self: *Fire) void {
        self.allocator.free(self.field);
        self.allocator.free(self.count);
        self.allocator.free(self.heater);
        self.allocator.free(self.hotplate);
    }

    fn idx(self: *Fire, r: usize, c: usize) i32 {
        return self.field[r * self.cols + c];
    }

    fn setIdx(self: *Fire, arr: []i32, r: usize, c: usize, val: i32) void {
        arr[r * self.cols + c] = val;
    }

    fn getCount(self: *Fire, r: usize, c: usize) i32 {
        return self.count[r * self.cols + c];
    }

    fn warm(self: *Fire) void {
        for (0..self.cols) |i| {
            self.hotplate[i] /= 2;
        }
        for (0..self.cols) |i| {
            self.hotplate[i] += self.heater[i] * @as(u8, @intCast(@min(255, self.max_temp)));
        }
    }

    fn nextFrame(self: *Fire) void {
        // Clear count buffer
        for (0..self.rows) |i| {
            for (0..self.cols) |j| {
                self.setIdx(self.count, i, j, 0);
            }
        }

        // Process from top to bottom (fire propagates upward)
        const h: usize = 0;

        for (h..self.rows + 1) |i| {
            var row_sum: i32 = 0;
            for (0..self.cols) |j| {
                var avg: f32 = 0;
                var counter: f32 = 0;

                // Search space: 7 wide x 5 tall, biased downward
                var xoff: i32 = -3;
                while (xoff <= 3) : (xoff += 1) {
                    var yoff: i32 = -1;
                    while (yoff <= 3) : (yoff += 1) {
                        var y: i32 = @as(i32, @intCast(i)) + yoff;
                        if (y < 0) y = 0;
                        const x: i32 = @as(i32, @intCast(j)) + xoff;

                        if (x < 0 or x >= @as(i32, @intCast(self.cols))) {
                            avg += 0;
                        } else if (y >= @as(i32, @intCast(self.rows))) {
                            avg += @floatFromInt(self.hotplate[@intCast(x)]);
                        } else {
                            avg += @floatFromInt(self.field[@as(usize, @intCast(y)) * self.cols + @as(usize, @intCast(x))]);
                        }
                        counter += 1;
                    }
                }

                avg /= counter;
                const cooled = cooldown(@intFromFloat(avg));
                if (i > 0) {
                    self.setIdx(self.count, i - 1, j, cooled);
                    row_sum += cooled;
                }
            }
            if (row_sum > 0 and i < self.height_record) {
                self.height_record = i;
            }
        }

        // Copy count back to field
        @memcpy(self.field, self.count);
    }

    fn isHot(self: *Fire, row: i32, col: i32) bool {
        if (col < 0 or col >= @as(i32, @intCast(self.cols))) return false;
        if (row < 0) return false;
        // Below screen = heat source, always hot
        if (row >= @as(i32, @intCast(self.rows))) return true;
        return self.field[@as(usize, @intCast(row)) * self.cols + @as(usize, @intCast(col))] > 0;
    }

    fn getEdgeType(self: *Fire, row: usize, col: usize) EdgeType {
        const r: i32 = @intCast(row);
        const c: i32 = @intCast(col);

        const above = self.isHot(r - 1, c);
        const below = self.isHot(r + 1, c);
        const left = self.isHot(r, c - 1);
        const right = self.isHot(r, c + 1);

        // Corner detection
        if (!above and !left and below and right) return .tl;
        if (!above and !right and below and left) return .tr;
        if (!below and !left and above and right) return .bl;
        if (!below and !right and above and left) return .br;

        // Edge detection
        if (!above and below) return .top;
        if (!below and above) return .bottom;
        if (!left and right) return .left;
        if (!right and left) return .right;

        return .internal;
    }

    fn pickRandomChar(chars: []const u8) u21 {
        var codepoint_count: usize = 0;
        const view = std.unicode.Utf8View.initUnchecked(chars);
        var count_iter = view.iterator();
        while (count_iter.nextCodepoint() != null) codepoint_count += 1;

        if (codepoint_count == 0) return '@';
        const pick = rand_int(codepoint_count);

        var iter = view.iterator();
        var pos: usize = 0;
        while (iter.nextCodepoint()) |cp| {
            if (pos == pick) return cp;
            pos += 1;
        }
        return '@';
    }

    fn getCharForEdge(edge: EdgeType, heat: i32, max_temp: i32) u21 {
        const pal = current_palette;
        return switch (edge) {
            .top => pickRandomChar(pal.edge_top),
            .bottom => pickRandomChar(pal.edge_bottom),
            .left => pickRandomChar(pal.edge_left),
            .right => pickRandomChar(pal.edge_right),
            .tl => pickRandomChar(pal.corner_tl),
            .tr => pickRandomChar(pal.corner_tr),
            .bl => pickRandomChar(pal.corner_bl),
            .br => pickRandomChar(pal.corner_br),
            .internal => blk: {
                // Pick internal char based on heat intensity - count codepoints first
                var codepoint_count: usize = 0;
                const view = std.unicode.Utf8View.initUnchecked(pal.internal);
                var count_iter = view.iterator();
                while (count_iter.nextCodepoint() != null) codepoint_count += 1;

                if (codepoint_count == 0) break :blk '@';
                // Hotter = denser char (first chars in palette are denser)
                const raw_idx = @as(usize, @intCast(@divTrunc(@as(i32, @intCast(codepoint_count)) * heat, max_temp + 1)));
                const char_idx = if (raw_idx >= codepoint_count) 0 else codepoint_count - 1 - raw_idx;

                var iter = view.iterator();
                var pos: usize = 0;
                while (iter.nextCodepoint()) |cp| {
                    if (pos == char_idx) break :blk cp;
                    pos += 1;
                }
                break :blk '@';
            },
        };
    }

    fn render(self: *Fire, writer: anytype) !void {
        // Move to home, set background
        try writer.print("\x1b[H\x1b[48;5;{d}m", .{PALETTE[0]});

        // Render full screen
        for (0..self.rows) |i| {
            for (0..self.cols) |j| {
                const heat = self.field[i * self.cols + j];
                if (heat == 0) {
                    try writer.writeByte(' ');
                } else {
                    const color_idx = @min(PALETTE_SZ, @as(usize, @intCast(@divTrunc(PALETTE_SZ * @as(usize, @intCast(heat)), @as(usize, @intCast(self.max_temp))))) + 1);
                    const edge_type = self.getEdgeType(i, j);
                    const char = getCharForEdge(edge_type, heat, self.max_temp);
                    var buf: [4]u8 = undefined;
                    const len = std.unicode.utf8Encode(char, &buf) catch 1;
                    try writer.print("\x1b[38;5;{d}m{s}", .{ PALETTE[color_idx], buf[0..len] });
                }
            }
            if (i < self.rows - 1) {
                try writer.writeAll("\x1b[K\n");
            }
        }

        try writer.writeAll("\x1b[0m");
    }

    fn update(self: *Fire) void {
        // Dynamic "breathing" effect
        self.shuffle += 1;
        if (self.shuffle >= 10) {
            self.bounce += 1;
            self.shuffle = 0;
        }
        self.max_temp = self.bounce;
        if (self.bounce > max_temp_cap + self.rand_height) {
            self.rand_height = @intCast(rand_int(3));
            self.bounce = min_temp - self.rand_height;
        }

        // Evolve heater using Wolfram Rule 60
        wolfram(self.heater, wolfram_rule);

        // Randomly flip a heater cell ~1 in 30 frames
        if (rand_int(30) == 0) {
            self.heater[rand_int(self.cols)] ^= 1;
        }

        self.warm();
        self.nextFrame();
    }
};

fn getTermSize() !struct { cols: usize, rows: usize } {
    var ws: posix.winsize = undefined;
    const rc = posix.system.ioctl(posix.STDOUT_FILENO, posix.T.IOCGWINSZ, @intFromPtr(&ws));
    if (rc == 0) {
        return .{ .cols = ws.col, .rows = ws.row };
    }
    return .{ .cols = 80, .rows = 24 };
}

fn selectPalette(name: []const u8) bool {
    if (std.mem.eql(u8, name, "blocks")) {
        current_palette = &PALETTES.blocks;
        return true;
    } else if (std.mem.eql(u8, name, "ascii")) {
        current_palette = &PALETTES.ascii;
        return true;
    } else if (std.mem.eql(u8, name, "dots")) {
        current_palette = &PALETTES.dots;
        return true;
    } else if (std.mem.eql(u8, name, "hash")) {
        current_palette = &PALETTES.hash;
        return true;
    }
    return false;
}

pub fn main() !void {
    // Parse command line args for palette selection
    var args = std.process.args();
    _ = args.skip(); // skip program name
    if (args.next()) |arg| {
        if (!selectPalette(arg)) {
            const msg = "Unknown palette. Available: blocks, ascii, dots, hash\n";
            _ = std.fs.File.stdout().writeAll(msg) catch {};
            return;
        }
    }

    prng = std.Random.DefaultPrng.init(@bitCast(std.time.milliTimestamp()));
    random = prng.random();

    // Set up SIGWINCH handler for terminal resize
    const act = posix.Sigaction{
        .handler = .{ .handler = sigwinchHandler },
        .mask = posix.sigemptyset(),
        .flags = 0,
    };
    posix.sigaction(posix.SIG.WINCH, &act, null);

    const stdout_file = std.fs.File.stdout();
    var buf: [16384]u8 = undefined;
    var stdout_writer = stdout_file.writer(&buf);
    const writer = &stdout_writer.interface;

    // Hide cursor, clear screen
    try writer.writeAll("\x1b[?25l\x1b[2J");
    try writer.flush();

    var gpa = std.heap.GeneralPurposeAllocator(.{}){};
    defer _ = gpa.deinit();
    const allocator = gpa.allocator();

    var size = try getTermSize();
    var fire = try Fire.init(allocator, size.cols, size.rows);
    defer fire.deinit();

    // Main loop
    while (true) {
        // Check for resize
        if (sig_resized.swap(false, .acquire)) {
            const new_size = try getTermSize();
            if (new_size.cols != size.cols or new_size.rows != size.rows) {
                size = new_size;
                fire.deinit();
                fire = try Fire.init(allocator, size.cols, size.rows);
                try writer.writeAll("\x1b[2J");
            }
        }

        fire.update();
        try fire.render(writer);
        try writer.flush();

        std.Thread.sleep(50 * std.time.ns_per_ms);
    }
}
