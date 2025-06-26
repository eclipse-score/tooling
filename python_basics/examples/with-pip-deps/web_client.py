#!/usr/bin/env python3
"""Example web client using pip dependencies."""

import click
import requests

@click.command()
@click.option("--url", default="https://httpbin.org/get", help="URL to fetch")
def main(url):
    """Simple web client demonstrating pip dependencies work."""
    click.echo(f"Fetching {url}...")
    try:
        response = requests.get(url, timeout=10)
        click.echo(f"Status: {response.status_code}")
        click.echo(f"Content length: {len(response.text)}")
    except requests.RequestException as e:
        click.echo(f"Error: {e}")
        return 1
    return 0

if __name__ == "__main__":
    main()