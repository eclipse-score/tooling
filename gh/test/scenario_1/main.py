from generated_github_client import Client


async def main():
    client = Client("your_token")
    result = await client.get_viewer()
    # Note: result is a dataclass!! with type validation!!
    print(result.viewer.login)

if __name__ == "__main__":
    import asyncio
    asyncio.run(main())
