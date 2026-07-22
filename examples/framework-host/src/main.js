import { mountRemote } from "@nori/framework";
import { mount } from "@nori/core";

const slot = document.querySelector("#slot") ?? document.body;

// Host loads the remote container (injected by MF runtime) and mounts it.
void mountRemote("framework_remote", slot, {
  module: "./App",
  mount: (component, el) => mount(component, el)
});
