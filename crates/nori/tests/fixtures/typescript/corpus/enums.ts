enum Color {
  Red = "red",
  Green = "green",
  Blue = "blue",
}

enum Mixed {
  A = 10,
  B,
  C = 20,
}

const enum Flags {
  None = 0,
  Read = 1,
  Write = 2,
}

const c = Color.Red;
const m = Mixed.B;
const f = Flags.Read;
