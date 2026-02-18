import { it, expect } from "vitest";
import { reduce, register, Envelope } from "./kernel";

it("replace reducer overwrites", () => {
  expect(reduce("replace", 1, 2)).toBe(2);
});

it("append concatenates arrays", () => {
  expect(reduce("append", [1], [2, 3])).toEqual([1, 2, 3]);
});

it("merge shallow merges objects", () => {
  expect(reduce("merge", { a: 1 }, { b: 2 })).toEqual({ a: 1, b: 2 });
});

it("patch applies object patch", () => {
  expect(reduce("patch", { a: 1, b: 2 }, { b: 3 })).toEqual({ a: 1, b: 3 });
});

it("allows custom reducer under allowlist name", () => {
  register("downsample", (_a, b) => (Array.isArray(b) ? b.slice(0, 1) : b));
  expect(reduce("downsample", [], [1, 2, 3])).toEqual([1]);
});

it("Envelope type enforces shape", () => {
  const e: Envelope<{ foo: string }> = {
    topic: "chat",
    seq: 1,
    ts: Date.now(),
    version: 1,
    data: { foo: "bar" }
  };
  expect(e.data.foo).toBe("bar");
});
