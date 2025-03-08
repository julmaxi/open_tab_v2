<script>
    import { page } from '$app/stores';
</script>

<style>
    body {
        background-color: #f8f9fa;
        font-family: Arial, sans-serif;
        margin: 0;
        padding: 0;
        display: flex;
        justify-content: center;
        align-items: center;
        height: 100vh;
    }
    .error-message {
        text-align: center;
        margin-top: 50px;
        padding: 20px;
        border: 1px solid #ddd;
        border-radius: 8px;
        background-color: #fff;
        box-shadow: 0 4px 8px rgba(0, 0, 0, 0.1);
    }
    h1 {
        color: #333;
        font-size: 24px;
        margin-bottom: 20px;
    }
    p {
        color: #666;
        font-size: 16px;
        margin-bottom: 20px;
    }
    .error-details {
        color: #e74c3c;
        font-size: 14px;
        background-color: #f9eaea;
        padding: 10px;
        border-radius: 4px;
        display: inline-block;
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
        <h1>Something went wrong</h1>

        <p>Send your tabmaster the following information:</p>

        <div class="error-details">
            {$page.error.message}
        </div>
    {/if}
</div>