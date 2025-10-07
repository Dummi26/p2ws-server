# p²ws

a pixelplace websocket server

# p² protocol

## Connections

The p² protocol assumes that clients can connect to the server
in such a way that a bidirectional channel is created through which bytes can be sent.

Each message must start with `0xFF`, messages cannot contain any other bytes with that value, and a message's end
can always be determined before reading a byte that does not belong to that message (with the exception of the Heartbeat).

## Messages

After a client has connected to the server, it must send an Authentication message.

Then, the client may send any number of Put messages optionally followed by a Disconnect Request
and (at the same time) the server may send any number of Update messages optionally followed by a Disconnect Request.

### Authentication

The client must send (in order):

- `0xFF A0`
- the byte-length of the username in UTF-8, minus one, as one byte (this requires `0 < length < 256`)
- the username encoded in UTF-8
- the user's one-time password (OTP) in 4 bytes
  + which OTP algorithm is chosen depends on the user and the server, and is not specified here
  + to convert 8 digits (each 0-9) to 4 bytes, encode each as a 4 bit binary number
  + if there are more or less digits, remove leading digits or add leading zeros
  + it is not required that every client is able to generate the OTP on its own,
    a client could simply ask the user to enter their OTP from an authenticator app on their phone

If a user authenticates twice with the same one-time password because it has not changed yet (usually it changes once every 30 seconds),
servers should treat all but the first authentication request as if the OTP was incorrect (in case someone is listening in on the connection but has not hijacked it).

### Put

The client may send (in order):

- `0xFF D0`
+ The `x` and `y` position of the pixel it wants to change, each encoded as 2 bytes (see #coordinate-encoding)
+ The color it wants to change the pixel to (see #color-encoding)

Note: It is not guaranteed that the server will set the pixel to the requested color. Clients which can set pixel and
display the canvas at the same time should not assume that a pixel has a certain color just because they have requested it.

### Sub

The client may send (in order):

- `0xFF AF`
+ The `x1` and `y1` values of the top left pixel coordinate, each encoded as 2 bytes (see #coordinate-encoding)
+ The `x2` and `y2` values of the bottom right pixel coordinate, each encoded as 2 bytes (see #coordinate-encoding)

This will tell the server to notify the client about updated pixels in the specified area.
If no Sub message is ever sent after Authenticating, the client will not receive any Update messages from the server.
NOTE: Once a Sub message is sent, servers may send Update messages for pixels within or even partially or entirely
outside the specified area. Clients should not assume that they will only receive updates they actually care about.

### Heartbeat

The client may send:

- `0xFF`

Clients should send this once every 50-60 seconds if they have not sent anything else.
If a client does not send anything for two minutes, servers may stop
sending updates to that client or discard its connection entirely.
Sending Put or Sub messages must also be enough for servers to keep the client's connection active.

## Disconnect Request

The server or client may send `0xFF 00` to indicate that the connection should be closed.

### Update

After receiving a Sub message from a client, the server may send (in order):

- `0xFF hw` where `0 < w <= 15, 0 <= h <= 7`
- The `x` and `y` position of the top left pixel of the area which it wants to update on the client
- The `w*(h+1)` colors of the pixel in the area defined by `x, y, w, h+1` (`h+1` rows, where each row contains `w` colors, and each color is 2 bytes)
  + Instead of a color, the special value `0x00 00` (decoded as the number `0`, which is not a valid color) can be used to indicate
    that the pixel has not actually changed, which some servers may use to increase the efficiency of the Update messages.

A simpler version of this (where `w = 1, h = 0`) to update only a single pixel:

- `0xFF 01`
- The `x` and `y` position of a pixel
- The pixel's color

## Coordinate Encoding

Let `n` be a number so that `-127 <= n <= 127`, then `bin_i8(n)` is the binary encoding of that number.

For `0 <= n <= 127`, `bin_i8(n)` is `0` followed by 7 bits encoding the number `n` in binary:

```
bin_i8(   0) = 0b00000000
bin_i8(   1) = 0b00000001
bin_i8( 127) = 0b01111111
```

For `-127 <= n < 1`, `bin_i8(n)` is `1` followed by 7 bits encoding the number `abs(n)-1` in binary:

```
bin_i8(-  1) = 0b10000000
bin_i8(-127) = 0b11111110
```

Let `x` be a number so that `-32512 <= x <= 32512`. (Note: `32512 = 127*255+127`)
Numbers in this range are coordinates. They can be encoded in two bytes:

If `-127 <= x <= 127`:

```
bytes[0] = 0
bytes[1] = bin_i8(x)
```

If `x > 127`:

```
bytes[0] = bin_i8((x+127)/255)
bytes[1] = bin_i8((x+127)%255-127)
```

If `x < -127`:

```
bytes[0] = bin_i8(-((|x|+127)/255))
bytes[1] = bin_i8(  (|x|+127)%255-127)
```

Note: `/` is flooring integer division, and `%` is the modulo or remainder operation (since both are only applied to positive numbers in the above formulas, choosing `mod` or `rem` does not make a difference)

## Color Encoding

A color is a rgb value, where `r`, `g`, and `b` are 5-bit numbers: `0 <= r, g, b <= 63`.

Let `x = 0b rrrr ggggg bbbbb` be a positive number created from the rgb components (without the most significant bit of `r`).

If `r < 32` (that is, the first of the 5 bits of `r` is `0`): Encode `x+1` as described in #coordinate-encoding.

If `r >= 32` (that is, the first of the 5 bits of `r` is `1`): Encode `-x-1` as described in #coordinate-encoding.

To decode, first decode `y` (see #coordinate-encoding). \
If `y > 0`, subtract one, then use the least significant 15 bits of the number as `rrrrrgggggbbbbb`. \
If `y < 0`, subtract one from its absolute value, add `16384`, then use the least significant 15 bits of that number as `rrrrrgggggbbbbb`.
