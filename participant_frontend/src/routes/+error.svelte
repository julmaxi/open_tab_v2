<script>
    import { page } from '$app/stores';
</script>

<style>
    h1 {
        color: #333;
        font-family: Arial, sans-serif;
    }
    .error-message {
        text-align: center;
        margin-top: 50px;
    }
</style>

<div class="error-message">
    {#if $page.status == 401 || $page.status == 403}
        <h1>
            You are not authorized to view this page.
            You may have to scan your QR code again.
        </h1>
    {:else if $page.status == 404}
        <h1>
            The page you are looking for does not exist.
        </h1>
    {:else}
        <h1>{$page.error.message}</h1>
    {/if}
</div>