<script>
    import Card from "$lib/Card.svelte";
    import CardButton from "$lib/CardButton.svelte";

    export let tournament;
</script>

<style>
    .list {
        display: flex;
        flex-direction: row;
        justify-content: center;
        align-items: center;
        flex-wrap: wrap;

        width: 100%;
    }
    
    .card {
        padding: 10px;
        border: 1px solid #aaa;
        font-weight: bold;
        align-items: center;
        line-height: 1rem;
        border-radius: 5px;
        margin-right: 5px;
        box-shadow: 0 0 0.25rem rgba(0, 0, 0, 0.25);
        background-color: white;
    }

    h2 {
        font-weight: bold;
    }

    h3 {
        font-weight: bold;
    }

    .container {
        max-width: 750px;
        margin: 0 auto;
        background-color: rgb(251, 250, 254);
    }

    nav {
        height: 3rem;
        padding: 0.25rem;
        display: flex;
        flex-direction: row;
        align-items: center;

        font-weight: bold;

        background-color: rgb(251, 250, 254);

        box-shadow: 0 0 0.25rem rgba(0, 0, 0, 0.25);

        position: sticky;
        top: 0;
    }

    .back {
        background-color: rgb(251, 250, 254);
        width: 100vw;
        height: 100vh;
    }

    .img-container {
        display: flex;
        flex-direction: row;
        justify-content: center;
        align-items: center;
    }

    .img-container img {
        margin-top: 8px;
        height: 50px;
    }
    
    .card-content {
        width: 15rem;
    }

    .card-content h3 {
        margin: 5px;
    }

    .card-content img {
        padding-bottom: 5px;
    }

    a {
        display: block;
    }
</style>

<a href={`/tournament/${tournament.tournament_uuid}`}>
<Card>
    <div class="card-content" slot="content">
        <h3>{tournament.name}</h3>

        {#if tournament.image}
            <div class="img-container">
                <img src={tournament.image} alt="Tournament Logo" />
            </div>
        {/if}
    </div>

    <div class="footer" slot="footer">
        {#if tournament.user_is_participant}
            <CardButton href={`/tournament/${tournament.tournament_uuid}`} label="View Your Page" />
        {/if}
        {#if tournament.show_participants}
            <CardButton href={`/tournament/${tournament.tournament_uuid}/participants`} label="View Participants" />
        {/if}
        {#if tournament.show_tab }
            <CardButton label="View Tab" href={`/tournament/${tournament.tournament_uuid}/tab`} />
        {/if}
    </div>
</Card>
</a>

