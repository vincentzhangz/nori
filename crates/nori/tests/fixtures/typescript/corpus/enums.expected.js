var Color; (function (Color) {
  Color["Red"] = "red";
  Color["Green"] = "green";
  Color["Blue"] = "blue";
})(Color || (Color = {}));
var Mixed; (function (Mixed) {
  Mixed[Mixed["A"] = 10] = "A";
  Mixed[Mixed["B"] = 11] = "B";
  Mixed[Mixed["C"] = 20] = "C";
})(Mixed || (Mixed = {}));
const c = Color.Red;
const m = Mixed.B;
const f = 1;
