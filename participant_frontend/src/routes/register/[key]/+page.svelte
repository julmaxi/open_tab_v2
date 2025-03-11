<script>
    export let data;
</script>

<style>
    div {
        background-color: rgb(251, 250, 254);
        min-width: 100vw;
        min-height: 100vh;

        display: flex;
        flex-direction: column;
        align-items: center;
        justify-content: center;

        padding-left: 1rem;
        padding-right: 1rem;
    }
    
    button {
        display: block;
        border-radius: 0.25rem;
        padding: 1rem;
        padding-top: 0.5rem;
        padding-bottom: 0.5rem;
        background-color: rgb(41, 35, 211);
        text-align: center;
        font-weight: bold;
        color: white;
        margin: 0.5rem;
    }

    button.secondary {
        background-color: rgb(211, 35, 35);
    }

    .warning {
        color: red;
        font-weight: bold;
    }

    h1 {
        font-size: 1.25rem;
        font-weight: bold;
    }

    h2 {
        font-weight: lighter;
    }
</style>

<div>
    <h1>Registration Page</h1>
    <h1>{data.participant_name}</h1>
    <h2>{data.tournament_name}</h2>

    {#if data.canClaimAsUser}
        <p>
            Do you want to claim this page for your account?
            You will then be able to access this page while logged in and the tournament will
            appear in your personal statistics.
            Alternatively, you can also access this tournament anonymously. This will log you out
            of your account.
        </p>

        <p>
            If you are not {data.participant_name}, please let your tabmaster know.
        </p>
        <form method="POST" action="?/registerAsUser">
            <input name="key" type="hidden" value="{data.key}" />
            <button>Claim</button>
        </form>

        <form method="POST" action="?/register">
            <input name="key" type="hidden" value="{data.key}" />
            <button class="secondary">Claim anonymously</button>
        </form>

        <p class="warning">
            If you claim this page anonymously, anyone with this url can log in to your account! Be sure to keep it secret.
        </p>
    {:else}
        <p>
            You are about to log in to your personal page for {data.tournament_name}. This will allow you
        to access your scores and submit ballots. If you ever get logged out, return to this page.
        </p>

        <p>
            If you are not {data.participant_name}, please let your tabmaster know.
        </p>
        <form method="POST" action="?/register">
            <input name="key" type="hidden" value="{data.key}" />
            <button id="link-login">Login</button>
        </form>

        <p class="warning">
            Anyone with this url can log in to your account! Be sure to keep it secret.
        </p>
    {/if}
</div>