<script>
    import Card from "$lib/Card.svelte";
    import CardButton from "$lib/CardButton.svelte";

    export let data;
</script>

<style>
    h1 {
        font-size: 1.25rem;
    }

    h2 {
        font-weight: bold;
    }
</style>

<div>
    <h1>
        Admin Page for {data.tournament_name}
    </h1>

    <h2>
        Rounds
    </h2>

    <div class="list">
        {#each data.rounds as round}
            <Card>
                <div slot="content">
                <h2>
                    {round.name}
                </h2>
                </div>
                <div slot="footer">
                    <CardButton href="/tournament/{data.tournament_id}/admin/round/{round.uuid}/presentation" label="Show Presenation" />                        
                </div>
            </Card>
        {/each}
    </div>
</div>