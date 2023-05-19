<script>
    import { enhance } from "$app/forms";
    import EditableBallot from "$lib/EditableBallot.svelte";
    export let data;
</script>

<form method="POST" use:enhance>
    <EditableBallot bind:ballot={data.ballot} />

    <button class="p-2 text-center bg-green-600 text-white w-full font-bold">Submit</button>
</form>