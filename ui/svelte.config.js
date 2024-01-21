import adapter from "@sveltejs/adapter-auto";
import preprocess from "svelte-preprocess";

const config = {
  kit: {
    adapter: adapter(),
  },
  preprocess: preprocess(),
};

export default config;
