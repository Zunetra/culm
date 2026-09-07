<script lang="ts">
  import { page } from "$app/state";
  import { resolve } from "$app/paths";

  // SvelteKit renders this in place of the route that failed. The status is
  // safe to show; the message is only meaningful for errors the application
  // raised deliberately, so anything else gets a fixed sentence.
  const message: string =
    page.status === 404 ? "That page does not exist." : "Something went wrong.";
</script>

<section>
  <h1>{page.status}</h1>
  <p>{message}</p>
  <a href={resolve("/")}>Back to the start</a>
</section>
