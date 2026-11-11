"""Minimal stand-in for tkinter so the v1.0.1 Python engine can be imported headless.

The harness never builds any widgets: it bypasses SoulscaleApp.__init__ and stubs every
UI method. These names only need to exist so `import tkinter as tk` and the
`from tkinter import filedialog, messagebox` line succeed.
"""


class TclError(Exception):
    pass


class _Widget:
    def __init__(self, *args, **kwargs):
        pass

    def __getattr__(self, name):
        def _noop(*args, **kwargs):
            return None

        return _noop


class Tk(_Widget):
    pass


class Toplevel(_Widget):
    pass


class Frame(_Widget):
    pass


class Button(_Widget):
    pass


class Label(_Widget):
    pass


class Canvas(_Widget):
    pass


class _Var:
    def __init__(self, *args, value=None, **kwargs):
        self._value = value

    def set(self, value):
        self._value = value

    def get(self):
        return self._value


class StringVar(_Var):
    pass


class IntVar(_Var):
    pass


class BooleanVar(_Var):
    pass
