import { describe, expect, it, vi } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { Segmented, Toggle } from "./ui";
import { toAppError, api } from "../services/api";

describe("ui primitives", () => {
  it("Toggle exposes switch semantics and reports the new value", async () => {
    const onChange = vi.fn();
    render(<Toggle checked={false} onChange={onChange} label="Mute" />);
    const sw = screen.getByRole("switch", { name: "Mute" });
    expect(sw).toHaveAttribute("aria-checked", "false");
    await userEvent.click(sw);
    expect(onChange).toHaveBeenCalledWith(true);
  });

  it("Segmented marks the selected option and reports changes", async () => {
    const onChange = vi.fn();
    render(
      <Segmented
        label="Theme"
        value="a"
        options={[{ value: "a", label: "A" }, { value: "b", label: "B" }]}
        onChange={onChange}
      />,
    );
    expect(screen.getByRole("radio", { name: "A" })).toHaveAttribute("aria-checked", "true");
    await userEvent.click(screen.getByRole("radio", { name: "B" }));
    expect(onChange).toHaveBeenCalledWith("b");
  });
});

describe("api", () => {
  it("normalises errors", () => {
    expect(toAppError({ code: "NotFound", message: "x" })).toEqual({ code: "NotFound", message: "x" });
    expect(toAppError("boom")).toEqual({ code: "Internal", message: "boom" });
  });

  it("fails honestly outside the desktop shell instead of faking data", async () => {
    await expect(api.bootstrap()).rejects.toMatchObject({ code: "Unsupported" });
  });
});
