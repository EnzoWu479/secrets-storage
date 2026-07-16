import { mount } from "@vue/test-utils";
import { describe, expect, it } from "vitest";

import UiIcon from "./UiIcon.vue";

const iconNames = [
  "lock",
  "vault",
  "eye",
  "copy",
  "google",
  "google-drive",
  "password",
  "api",
  "token",
  "note",
  "ssh",
  "settings",
  "sync",
  "warning",
] as const;

describe("UiIcon", () => {
  it("monta um SVG inline que herda a cor e usa o tamanho padrão", () => {
    const svg = mount(UiIcon, { props: { name: "lock" } }).get("svg");

    expect(svg.attributes()).toMatchObject({
      width: "20",
      height: "20",
      fill: "none",
      stroke: "currentColor",
      "stroke-width": "1.75",
      "aria-hidden": "true",
    });
  });

  it.each(iconNames)("renderiza o ícone %s com paths locais", (name) => {
    const wrapper = mount(UiIcon, { props: { name } });
    const paths = wrapper.findAll("path");

    expect(wrapper.find("svg").exists()).toBe(true);
    expect(wrapper.get("svg").attributes("data-icon")).toBe(name);
    expect(paths.length).toBeGreaterThan(0);
    expect(paths.every((path) => Boolean(path.attributes("d")))).toBe(true);
  });
});
