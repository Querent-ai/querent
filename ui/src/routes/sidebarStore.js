import { writable } from "svelte/store";

export const showCollectorComponent = writable(false);

showCollectorComponent.subscribe((value) => {
  console.log("showCollectorComponent:", value);
});
