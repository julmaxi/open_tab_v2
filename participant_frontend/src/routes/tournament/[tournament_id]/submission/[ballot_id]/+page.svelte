<script>
    import EditableBallot from "$lib/EditableBallot.svelte";
    export let data;

    import { page } from '$app/stores'
    import { enhance } from "$app/forms";
    import BellAnimation from "$lib/BellAnimation.svelte";
    import LoadingModal from "$lib/LoadingModal.svelte";
    $: shouldEdit = $page.url.searchParams.get('edit') === "1";
    const debateId = data.debate.uuid;

    const tournamentId = data.tournamentId;

	let isSubmitting = false;
</script>


<form method="POST" action={`/tournament/${tournamentId}/debate/${debateId}`} use:enhance={
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
    <EditableBallot bind:ballot={data.ballot} compact={!shouldEdit} />

    {#if shouldEdit}
        <div class="flex">
            <a class="grow p-2 text-center bg-gray-500 text-white w-full font-bold" href="?edit=0">Cancel</a>
            <button class="grow p-2 text-center bg-green-500 text-white w-full font-bold">Submit</button>
        </div>
    {:else}
        <a class="block p-2 text-center bg-gray-500 text-white w-full font-bold" href="?edit=1">Edit</a>
    {/if}
</form>