"""Read only the native cursor image, or move the pointer for an Xvfb smoke test."""
import ctypes as c
import ctypes.util
import json
import sys


def main():
    x11 = c.CDLL(ctypes.util.find_library("X11"))
    x11.XOpenDisplay.argtypes = [c.c_char_p]
    x11.XOpenDisplay.restype = c.c_void_p
    display = x11.XOpenDisplay(None)
    if not display:
        raise RuntimeError("X11 display unavailable")
    x11.XCloseDisplay.argtypes = [c.c_void_p]
    try:
        if sys.argv[1] == "warp":
            x11.XDefaultRootWindow.argtypes = [c.c_void_p]
            x11.XDefaultRootWindow.restype = c.c_ulong
            x11.XWarpPointer.argtypes = [c.c_void_p,c.c_ulong,c.c_ulong,c.c_int,c.c_int,c.c_uint,c.c_uint,c.c_int,c.c_int]
            x11.XWarpPointer(display,0,x11.XDefaultRootWindow(display),0,0,0,0,int(sys.argv[2]),int(sys.argv[3]))
            x11.XSync.argtypes = [c.c_void_p,c.c_int]
            x11.XSync(display,0)
            return

        class Cursor(c.Structure):
            _fields_ = [("x",c.c_short),("y",c.c_short),("width",c.c_ushort),("height",c.c_ushort),
                        ("xhot",c.c_ushort),("yhot",c.c_ushort),("serial",c.c_ulong),
                        ("pixels",c.POINTER(c.c_ulong)),("atom",c.c_ulong),("name",c.c_char_p)]
        fixes = c.CDLL(ctypes.util.find_library("Xfixes"))
        fixes.XFixesGetCursorImage.argtypes = [c.c_void_p]
        fixes.XFixesGetCursorImage.restype = c.POINTER(Cursor)
        pointer = fixes.XFixesGetCursorImage(display)
        if not pointer:
            raise RuntimeError("No native cursor image")
        try:
            cursor = pointer.contents
            actual = [cursor.width,cursor.height,cursor.xhot,cursor.yhot]
            expected = list(map(int,sys.argv[2:6]))
            assert actual == expected, f"native cursor dimensions/hotspot {actual}, expected {expected}, pointer at ({cursor.x}, {cursor.y})"
            colors = [cursor.pixels[i] for i in range(cursor.width*cursor.height)]
            dark = sum((p>>24)&255 > 200 and max((p>>16)&255,(p>>8)&255,p&255)<120 for p in colors)
            white = sum((p>>24)&255 > 200 and min((p>>16)&255,(p>>8)&255,p&255)>220 for p in colors)
            assert dark>10 and white>10, "cursor must contain the dark glyph and white contrast outline"
            print(json.dumps({"native_cursor":actual,"dark_pixels":dark,"white_pixels":white}))
        finally:
            x11.XFree.argtypes = [c.c_void_p]
            x11.XFree(pointer)
    finally:
        x11.XCloseDisplay(display)


if __name__ == "__main__":
    main()
