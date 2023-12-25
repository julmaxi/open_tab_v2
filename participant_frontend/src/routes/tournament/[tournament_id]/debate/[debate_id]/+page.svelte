<script>
    import { enhance } from "$app/forms";
    import EditableBallot from "$lib/EditableBallot.svelte";
    import LoadingModal from "$lib/LoadingModal.svelte";
    export let data;

    let isSubmitting = false;
</script>

<form method="POST" use:enhance={
    async () => {
        isSubmitting = true;

        return async ({ update }) => {
            await update();
            isSubmitting = false;
        };
    }
}>
    {#if isSubmitting}
        <LoadingModal />
    {/if}
    <EditableBallot bind:ballot={data.ballot} />

    <button class="p-2 text-center bg-green-600 text-white w-full font-bold">Submit</button>
</form>