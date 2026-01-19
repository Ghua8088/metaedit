import click
from . import MetadataEditor

@click.command()
@click.argument('exe_path', type=click.Path(exists=True))
@click.option('--icon', help='Path to .ico file')
@click.option('--version', help='Version string (e.g. 1.0.0.0)')
@click.option('--company', help='Company Name')
@click.option('--description', help='File Description')
@click.option('--product', help='Product Name')
@click.option('--copyright', help='Legal Copyright')
def main(exe_path, icon, version, company, description, product, copyright):
    """Simple CLI to edit PE metadata."""
    editor = MetadataEditor(exe_path)
    
    if icon:
        editor.set_icon(icon)
    if version:
        editor.set_version(version)
    if company:
        editor.set_string("CompanyName", company)
    if description:
        editor.set_string("FileDescription", description)
    if product:
        editor.set_string("ProductName", product)
    if copyright:
        editor.set_string("LegalCopyright", copyright)
        
    editor.apply()
    click.echo(f"Successfully updated metadata for {exe_path}")

if __name__ == "__main__":
    main()
