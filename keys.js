// Keys and mouse buttons to the core's key bits, after the realtime kit's
// input.js. A key pressed and let go between two frames still reaches the
// core once (`tapped`). The right mouse button holds the scope.
export function makeKeys(controls, names, target) {
  const bits = Object.fromEntries(names);
  const state = { held: 0, tapped: 0 };
  const bitOfKey = (code) => bits[controls.keys[code]] || 0;
  const bitOfButton = (button) => bits[controls.mouse[String(button)]] || 0;
  const press = (bit) => {
    state.held |= bit;
    state.tapped |= bit;
  };

  window.addEventListener("keydown", (e) => {
    const bit = bitOfKey(e.code);
    if (!bit) return;
    e.preventDefault();
    if (!e.repeat) press(bit);
  });
  window.addEventListener("keyup", (e) => {
    state.held &= ~bitOfKey(e.code);
  });
  target.addEventListener("pointerdown", (e) => press(bitOfButton(e.button)));
  window.addEventListener("pointerup", (e) => {
    state.held &= ~bitOfButton(e.button);
  });
  target.addEventListener("contextmenu", (e) => e.preventDefault());
  window.addEventListener("blur", () => {
    state.held = 0;
  });

  return {
    read() {
      const out = state.held | state.tapped;
      state.tapped = 0;
      return out;
    },
  };
}
