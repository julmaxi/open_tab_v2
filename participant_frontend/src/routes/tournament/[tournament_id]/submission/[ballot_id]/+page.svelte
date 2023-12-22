<script>
    import EditableBallot from "$lib/EditableBallot.svelte";
    export let data;

    import { page } from '$app/stores'
    import { enhance } from "$app/forms";
    $: shouldEdit = $page.url.searchParams.get('edit') === "1";
    const debateId = data.debate.uuid;

    const tournamentId = data.tournamentId;

</script>

<form method="POST" action={`/tournament/${tournamentId}/debate/${debateId}`} use:enhance>
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