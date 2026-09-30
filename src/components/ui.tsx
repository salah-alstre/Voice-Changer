import { useId, type ReactNode } from "react";
import { clamp } from "../utils/format";
import type { DeviceInfo } from "../types";

export function cx(...a: (string | false | null | undefined)[]): string {
  return a.filter(Boolean).join(" ");
}

export function Card({ children, className, ...rest }: { children: ReactNode; className?: string } & React.HTMLAttributes<HTMLDivElement>) {
  return (
    <div className={cx("card p-4", className)} {...rest}>
      {children}
    </div>
  );
}

export function PageHeader({ title, subtitle, actions }: { title: string; subtitle?: string; actions?: ReactNode }) {
  return (
    <div className="mb-4 flex items-start justify-between gap-4">
      <div className="min-w-0">
        <h1 className="m-0 text-[22px] font-semibold tracking-tight">{title}</h1>
        {subtitle && <p className="mt-1 mb-0 text-muted">{subtitle}</p>}
      </div>
      {actions && <div className="flex flex-wrap items-center justify-end gap-2">{actions}</div>}
    </div>
  );
}

export function SectionTitle({ children, right }: { children: ReactNode; right?: ReactNode }) {
  return (
    <div className="mb-3 flex items-center justify-between gap-2">
      <h2 className="m-0 text-[11px] font-semibold uppercase tracking-[0.09em] text-muted">{children}</h2>
      {right}
    </div>
  );
}

export function Toggle({
  checked, onChange, label, disabled,
}: { checked: boolean; onChange: (v: boolean) => void; label: string; disabled?: boolean }) {
  return (
    <button
      type="button"
      role="switch"
      aria-checked={checked}
      aria-label={label}
      title={label}
      disabled={disabled}
      className="toggle"
      onClick={() => onChange(!checked)}
    />
  );
}

export function Row({ label, hint, children }: { label: string; hint?: string; children: ReactNode }) {
  return (
    <div className="flex items-center justify-between gap-4 border-b border-line py-2.5 last:border-b-0">
      <div className="min-w-0">
        <div className="font-medium">{label}</div>
        {hint && <div className="mt-0.5 text-[12px] text-muted">{hint}</div>}
      </div>
      <div className="flex flex-none items-center gap-2">{children}</div>
    </div>
  );
}

export function Slider({
  label, value, min, max, step = 1, onChange, format, disabled, className, defaultValue,
}: {
  label: string; value: number; min: number; max: number; step?: number;
  onChange: (v: number) => void; format?: (v: number) => string; disabled?: boolean;
  className?: string; defaultValue?: number;
}) {
  const id = useId();
  const pos = `${clamp(((value - min) / (max - min)) * 100, 0, 100)}%`;
  return (
    <div className={cx("min-w-0", className)}>
      <div className="mb-0.5 flex items-baseline justify-between gap-2 text-[12px]">
        <label htmlFor={id} className="truncate text-muted">{label}</label>
        <span className="font-mono tabular-nums">{format ? format(value) : value}</span>
      </div>
      <input
        id={id}
        type="range"
        className="slider"
        style={{ ["--pos" as string]: pos }}
        min={min}
        max={max}
        step={step}
        value={value}
        disabled={disabled}
        onChange={(e) => onChange(Number(e.target.value))}
        onDoubleClick={() => defaultValue !== undefined && onChange(defaultValue)}
      />
    </div>
  );
}

export function Segmented<T extends string>({
  value, options, onChange, label,
}: { value: T; options: { value: T; label: string }[]; onChange: (v: T) => void; label: string }) {
  return (
    <div role="radiogroup" aria-label={label} className="inline-flex rounded-[10px] border border-line bg-bg2 p-0.5">
      {options.map((o) => (
        <button
          key={o.value}
          type="button"
          role="radio"
          aria-checked={value === o.value}
          onClick={() => onChange(o.value)}
          className={cx(
            "cursor-pointer rounded-[8px] border-0 px-3 py-1 text-[12.5px] font-medium transition-colors",
            value === o.value ? "bg-accent text-white" : "bg-transparent text-muted hover:text-fg",
          )}
        >
          {o.label}
        </button>
      ))}
    </div>
  );
}

export function Chip({ kind, children, pulse }: { kind?: "live" | "ok" | "warn"; children: ReactNode; pulse?: boolean }) {
  return (
    <span className={cx("chip", kind === "live" && "chip-live", kind === "ok" && "chip-ok", kind === "warn" && "chip-warn")}>
      {pulse && <span className="pulse inline-block h-1.5 w-1.5 rounded-full bg-current" />}
      {children}
    </span>
  );
}

export function DeviceSelect({
  devices, value, onChange, label, allowDefault, defaultLabel, disabled, className,
}: {
  devices: DeviceInfo[]; value: string | null | undefined; onChange: (id: string | null) => void;
  label: string; allowDefault?: boolean; defaultLabel?: string; disabled?: boolean; className?: string;
}) {
  const active = devices.filter((d) => d.state === "active");
  const missing = value && !active.some((d) => d.id === value);
  return (
    <select
      aria-label={label}
      title={label}
      className={cx("input", className)}
      disabled={disabled}
      value={value ?? ""}
      onChange={(e) => onChange(e.target.value || null)}
    >
      {allowDefault && <option value="">{defaultLabel ?? "System default"}</option>}
      {missing && <option value={value!}>(unavailable)</option>}
      {active.map((d) => (
        <option key={d.id} value={d.id}>
          {d.name}
          {d.isDefault ? " ★" : ""}
        </option>
      ))}
    </select>
  );
}

export function Empty({ icon, title, text, action }: { icon?: ReactNode; title: string; text?: string; action?: ReactNode }) {
  return (
    <div className="flex flex-col items-center justify-center gap-2 px-6 py-12 text-center text-muted">
      {icon}
      <div className="text-[15px] font-medium text-fg">{title}</div>
      {text && <div className="max-w-md">{text}</div>}
      {action}
    </div>
  );
}

export function Banner({ kind, children }: { kind: "warn" | "error" | "info"; children: ReactNode }) {
  const color = kind === "error" ? "var(--danger)" : kind === "warn" ? "var(--warn)" : "var(--accent2)";
  return (
    <div
      role={kind === "info" ? "status" : "alert"}
      className="mb-3 rounded-xl border px-3.5 py-2.5 text-[13px]"
      style={{
        borderColor: `color-mix(in srgb, ${color} 55%, var(--line))`,
        background: `color-mix(in srgb, ${color} 10%, var(--panel))`,
      }}
    >
      {children}
    </div>
  );
}
