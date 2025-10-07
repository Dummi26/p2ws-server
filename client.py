import random
import time
from websockets.sync.client import connect
import pygame

# set to py2 for the second client
username = "py1"
user_otp = 1234

username = username.encode("utf-8")
if len(username) == 0 or len(username) > 255:
    exit(1)

window_size = (0, 0)
top_left = (0, 0)
bottom_right = (0, 0)
top_left_px = (0, 0)
zoom = 10

def enc_coord_i16(x):
    if -127 <= x and x <= 127:
        return (0, enc_coord_i8(x))
    elif x > 127:
        return (enc_coord_i8((x+127)//255), enc_coord_i8((x+127)%255-127))
    else:
        return (enc_coord_i8(-((abs(x)+127)//255)), enc_coord_i8((abs(x)+127)%255-127))

def dec_coord_i16(v, w):
    v = dec_coord_i8(v)
    w = dec_coord_i8(w)
    if v == 0:
        return w
    elif v > 0:
        return (v * 255) + (w + 127) - 127
    else:
        return -((abs(v) * 255) + (w + 127) - 127)

def enc_coord_i8(x):
    if 0 <= x and x <= 127:
        return x
    else:
        return abs(x)+127

def dec_coord_i8(v):
    if 0 <= v and v <= 127:
        return v
    else:
        return -(v-127)

def enc_color(r, g, b):
    x = ((r & 0xF) << 10) | ((g & 0x1F) << 5) | (b & 0x1F)
    if r & 0x10 == 0:
        return enc_coord_i16(x+1)
    else:
        return enc_coord_i16(-x-1)

def dec_color(v, w):
    y = dec_coord_i16(v, w)
    z = y & 0x7FFF
    if y < 0:
        z = abs(y) + 16383
    return ((z & 0x7C00) >> 10, (z & 0x03E0) >> 5, z & 0x001F)

def msg_put(x, y, r, g, b):
    message = bytearray(8)
    message[0] = 0xFF
    message[1] = 0xD0
    message[2], message[3] = enc_coord_i16(x)
    message[4], message[5] = enc_coord_i16(y)
    message[6], message[7] = enc_color(r, g, b)
    return message

with connect("ws://localhost:8080") as websocket:
    auth_message = bytearray(7 + len(username))
    auth_message[0] = 0xFF
    auth_message[1] = 0xA0
    auth_message[2] = len(username) - 1
    for i in range(0, len(username)):
        auth_message[3+i] = username[i]
    for i in range(1, 4):
        auth_message[-i] = (user_otp % 10) | (((user_otp // 10) % 10) << 4);
        user_otp = user_otp // 100
    websocket.send(auth_message)

    pygame.init()
    screen = pygame.display.set_mode((1280, 720), pygame.RESIZABLE)
    clock = pygame.time.Clock()
    running = True
    while running:
        for event in pygame.event.get():
            if event.type == pygame.QUIT:
                running = False
            elif event.type == pygame.MOUSEMOTION:
                mx, my = pygame.mouse.get_pos()
                tlx, tly = top_left_px
                x, y = (top_left[0] + (mx - tlx) // zoom, top_left[1] + (my - tly) // zoom)
                websocket.send(msg_put(x, y, random.randint(0, 32), random.randint(0, 32), random.randint(0, 32)))

        w, h = screen.get_size()
        if (w, h) != window_size:
            window_size = (w, h)
            screen.fill("black")
            top_left = (-w // zoom // 2, -h // zoom // 2)
            tlx, tly = top_left
            bottom_right = (-tlx, -tly)
            top_left_px = (w // 2 + tlx * zoom - zoom // 2, h // 2 + tly * zoom - zoom // 2)
            sub_message = bytearray(10)
            sub_message[0] = 0xFF
            sub_message[1] = 0xAF
            sub_message[2], sub_message[3] = enc_coord_i16(top_left[0])
            sub_message[4], sub_message[5] = enc_coord_i16(top_left[1])
            sub_message[6], sub_message[7] = enc_coord_i16(bottom_right[0])
            sub_message[8], sub_message[9] = enc_coord_i16(bottom_right[1])
            websocket.send(sub_message)

        for i in range(0, 10):
            timedOut = False
            try:
                for message in websocket.recv(timeout=0, decode=False).split(b'\xFF'):
                    if len(message) > 0:
                        if message[0] & 0b10000000 == 0:
                            w = message[0] & 0x0F
                            h = ((message[0] & 0x70) >> 4) + 1
                            x1 = dec_coord_i16(message[1], message[2])
                            y1 = dec_coord_i16(message[3], message[4])
                            byte_index = 5
                            for y_rel in range(0, h):
                                for x_rel in range(0, w):
                                    y = y1 + y_rel
                                    x = x1 + x_rel
                                    if (message[byte_index] | message[byte_index + 1]) != 0:
                                        r, g, b = dec_color(message[byte_index], message[byte_index + 1])
                                        byte_index += 2
                                        pygame.draw.rect(screen, (r * 8, g * 8, b * 8), (top_left_px[0]+zoom*(x-top_left[0]), top_left_px[1]+zoom*(y-top_left[1]), zoom, zoom))
                        else:
                            print("Received unknown message")
            except Exception:
                timedOut = True
            if timedOut:
                break

        pygame.display.flip()
        clock.tick(60)

    pygame.quit()
