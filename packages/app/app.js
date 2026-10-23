import { ping, echo } from 'comparison:bench/host';
export function scalar(value) { return (value + 1) >>> 0; }
export function floating(value) { return value * 1.25 + 0.5; }
export function text(value) { return value; }
export function bytes(value) { return value; }
export function structured(value) {
  return { count: (value.count + 1) >>> 0, scale: value.scale * 1.25, label: value.label };
}
export function imports(count) {
  let value = 0;
  for (let i = 0; i < count; i++) value = ping(value);
  return value;
}
export function importText(value) { return echo(value); }
