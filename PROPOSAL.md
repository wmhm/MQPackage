# MacroQuest / RedGuides Package Manager

*This document was originally posted as a [GitLab Snippet](https://gitlab.com/-/snippets/2256782).*

***

Currently installing things for MacroQuest is generally done by one of two ways:

- Manually copying files into their final location.
- Using something like the RedGuides Launcher to automate fetching files and copying
  them into the final location.

This suffers from a number of problems:

- There is no ability to version anything, leaving people to come up with adhoc solutions
  like throwing the version number in comments, a varible, or using dates ala RedGuides.
  - The first two have problems in that they're entirely optional, and entirely unstructured,
    dates have a problem in that there's no ability to encode any relevant information (say
    for beta versions, or if you want to have multiple independent lines of developer for bugfixes
    for an older version).

- There's no ability to handle interdependencies between packages in any sort of systemic fashion,
  instead relying on adhoc solutions (again) such as documenting that certain things must be
  installed, or just bundling dependencies (which can get particularly ugly when multiple things
  are bundling slightly different versions of something).

- There's no mechanism for detecting if a user has made modifications to a file, and if so what
  should be done for that file. Instead the default is typically just blindly overwrite any
  existing files, possibly blowing away custom changes made by the user.

- Similarly, there's no method for uninstall. This means that files can get left behind if
  something gets refactored to remove one of the files that it used previously. This also ends
  up causing problems if something that shipped with the default distribution gets removed
  completely.

- Things that are included in the default distribution cannot be independently upgraded (or
  at least, does not by default). This means that upgrades to included packages are locked
  behind updates to the whole of MQ. Possibly with something like the RedGuides launcher, it
  would be possible to watch the individual resources and manually update them that way.

Thus I propose to introduce a MacroQuest Packaging toolchain which seeks to solve all of
these problems.


## Format

The packaging format will be a simple ZIP archive, where members must be compressed using the
standard deflate algorithm.

The filename of the archive must be ``{name}-{version}.mq.zip``.

The file layout is very simple, the zip archive must match the layout of the MQ folder for
whatever files that the zip archive contains. For example:

```
.
├── config
│   └── MQ2Example.yml
├── lua
│   └── example.lua
└── plugins
    └── MQ2Example.dll
```


The paths inside of the zip file must be treated as case insensitive, and it is an error to
have a duplicated entry when looked at case insensitively. Empty folders are ignored, and
do not need to be included inside of the zip file. It is an error to have a directory named
``pkgdb`` at the top level of the package format, as this directory is reserved for the
package manager.

All package files must also have a top level ``metadata.yml`` file, which is used to hold
all of the metadata for this package.


## metadata.yml

The ``metadata.yml`` file operates as the metadata file both inside of a package, and as a
an entry in the database of installed packages. It is a yaml file with the following
schema:

```yaml
meta:
  name: "The Name of the package"
  version: "The version of the package"
  dependencies:
    dependency: version specifer
    dependency: version specifier

config_files:
  - relative path with glob
  - relative path with glob

files:
  - relative path
  - relative path
```

### name

The name of the package, this could be something like ``MQ2Forage``, ``KissAssist``, etc.

This is considered to be case insensitive, can only contain alphanumeric letters, and
must begin with a letter.


### version

The version number of this package, must follow the [Semver](https://semver.org/) spec.


### dependencies

A mapping of dependency name to version specifier. The version specifier constrains
the dependency version so that the dependency must match the constraint. It supports
the following operations:

- ``*`` - Matches any version number.
- ``=`` - "Exact" match (however if you have less than 3 digits in the specifier, it
          actually compiles to a range, i.e. ``=I.J`` is equal to ``>=I.J.0, <I.(J+1).0``).
- ``>`` - Greater than.
- ``>=`` - Greater than equal.
- ``<`` - Less than.
- ``<`` - Less than equal.
- ``~`` - Patch upgrades, this let's the "patch" part upgrade, so ``~I.J.K`` is the
          same as ``>=I.J.K, <I.(J+1).0``.
- ``^`` - "Compatible" upgrades, this allows any version part that are right of the
          first non zero part of the version to increase, but anything to the left
          of that must remain fixed.
- ``*`` - Wildcard, this is basically the same as ``=`` with less than 3 digits,
          so ``I.J.*`` is the same as ``=I.J``.

For more details, you can check out the Rust
[Version Op page](https://docs.rs/semver/latest/semver/enum.Op.html).

### config_files (Optional)

A list of file paths (relative to the MacroQuest directory) that are config files
that are "owned" by this package. They do not have to exist in the package itself
but can be created at runtime. This supports standard glob syntax so ``**`` is a
wildcard that matches directories recursively, and ``*`` is a wildcard that does
not traverse the directory boundary.

This is optional, if it doesn't exist then the packaging system will not be aware
of any config files that might belong to this package, so a "purge" (uninstall
with config removal) will leave those files behind.

### files (Optional)

A list of all files (relative to the MacroQuest directory) that belong to this
package. This is optional because for ease of use, it does not need to be specified
inside of a package itself (all files in the package are included by default),
however it is *not* optional when installed into the ``pkgdb`` directory.

This does not support the glob syntax, all files must be declared explicitly.


## mqpkg.yml

This file is used to configure the installer so that it is able to use the
correct settings for a particular target directory. It must exist at the root
of the target directory. It is a yaml file with a schema like:

```yaml
repositories:
  - https://example.com/live/packages.json
```

Multiple repositories can be listed, and if multiple are the installer will treat
them as if they were all a single repository with the combined set of all packages
available in all of the repositories.


## Operations

### Installing

Installing a package is fairly simple, all an installer needs to do is:

1. Unpack the archive.
2. Iterate over all of the content, copying it into it's respective location
   in the target directory EXCEPT for ``metadata.yml``.
3. Copy the ``metadata.yml`` into a ``pkgdb/$name/`` directory, where ``$name``
   is the normalized name of the package.
4. Generate a ``hashes.yml`` file, which is a mapping of file path, relative to
   the target directory, to a hex encoded sha256 digest. This should include the
   ``metadata.yml`` file and every other file installed by this package other
   than ``hashes.yml`` itself.


### Uninstalling

Uninstalling is also faily simple, all an installer needs to do is:

1. Locate the ``metadata.yml`` for the desired package and remove all of the
   files listed in it. The hashes listed in ``hashes.yml`` may be used to
   determine if the user has modified the file in any way. If they had then
   the installer can bail out or ask the user what it should do.

   The ``config/`` directory may be treated specially, and these files may
   be left behind by the installer. Optionally if the ``metadata.yml`` has
   any config file patterns listed in it, the installer may offer an option
   to purge configuration as well.

2. Remove the ``hashes.yml`` file and the ``pkgdb/$name/`` directory.



### Upgrades

Upgrades are a combination of first uninstalling the old version of the package,
then installing the new version.


## Repository Support

TODO: This uses JSON since it's not intended to be human editable, just readable
      and JSON is a more common API format than YAML. We could just use YAML
      though to keep the same format?

A repository is basically just a single URL that has a package manifest available
at it. A package manifest is just a single document that holds all of the metadata
for a number of projects (without any of the actual files for the package) so that
the installer can resolve a dependency set to determine what versions to install
of what.

The manifest has a schema like:

```json
{
    "meta": {
        "name": "A display name for this repository"
    },
    "packages": {
        "package name" {
            "version": {
                "dependencies": {
                    "dependency": "specifier",
                    "dependency": "specifier"
                },
                "urls": [
                    "an url to download this package at",
                    "another url, equivilant to the first"
                ]
                "digests": {
                    "sha256": "..."
                }
            },
        }
    }
}
```

The package name, version, and dependency information is the same as in the
``metadata.yml`` file.

The urls key is a list of URLs that this package can be downloaded from.

Digests is a mapping of hashing algorithm to hex encoded digest of the file
itself.

The meta.name is just a display name for this repository for better presentation
in the UI.

Essentially, an installer is intended to ingest the manifest file above, resolve
the files that have been requested to be installed into a set of packages and
their dependencies that satisify all of the constraints, then download the package
files from one of the URLs (falling back to others if the first one fails), check
that the digest matches, then go through and install all of the packages.


## FAQ

### Is this RedGuides specific?

This is primarily being aimed for RedGuides, however there is nothing in this proposal
that *requires* RedGuides or is otherwise specific to it. The intent is that the client
side tooling will be open source, and designed such that anything RedGuides specific is
pluggable.

The serverside code that generates the repository manifest for RedGuides will be specific
to RedGuides, so anyone else that wishes to use this will need to write their own code
to generate the repository manifest.


### What is a "Target" directory?

Essentially a target directory is just a MacroQuest install (or where MacroQuest will get
installed into). So if you want to have everything installed into ``C:\MacroQuest``, then
that is your target directory.

The package manager is agnostic as to live vs test vs emu, and can support multiple target
directories on the same system, each with their own set of repositories to pull from.


### What does the workflow look like for a RedGuides package author?

The workflow should hopefully be very similiar to what exists now. Essentially they
have to create a zip file that matches the format above with a proper ``metadata.yml``,
then they need to upload that using RedGuide's Resource Manager.

From there, the RedGuides server software will, behind the scenes, take that and add it
into it's repository manifest to make that package available for download through the
installer.

So roughly speaking, the primary difference is they'll just have to ensure that the
thing they upload is a valid package format, and the tooling can provide helpers to
generate that.


### What does workflow look like for an end user?

The end user will need to create the target directory, which will need to contain a
``mqpkg.yml``file at the *root* of the target directory. The installer can then
be invoked on the command line using something like (cli name tbd):

```
$ mq-package install --target C:\MacroQuest\ kissassist MQ2Shaman lootly
```

The installer itself will be designed to also be programatially invoked, so that
it can integrated within things like the launcher, or possibly even an in game
lua ui.


### What kind of files can be included in this?

TODO: Figure out if packaging/upgrading MacroQuest via this actually makes sense,
      or if that opens up too many worms for special case behavior. Maybe something
      like a "frozen" or unmanaged package could be considered? Basically we don't
      want an old plugin to mess up somebody's patch day because the system won't
      let them upgrade because it hasn't been recompiled yet.

Anything really! Including MacroQuest itself, MQ2Nav meshes, etc. The packaging
format is completely agnostic as to what is contained within the actual package,
it just sees them as opaque files that it's shuffling around into the correct
place and managing.

It is recommended that MacroQuest *is* packages in this system, and uses a date
based version, something like ``YYYYMMDD.N.Z``. Where ``YYYYMMDD`` reflects the
date of the ``eqgame.exe`` that the MacroQuest is compiled for, and ``N.Z`` is
used to denote releases within that date. So the 3rd release for the Feb 15, 2022
patch, would look something like: ``20220215.3.0``.

This allows packages that have to be recompiled against MacroQuest for each patch
to specify a dependency like ``MacroQuest: "=YYYYMMDD"`` so that the package
system can ensure that MacroQuest and it's plugins are compatible with each other.

Another interesting case is that MQ2Nav meshes can be packaged within this system,
allowing for easy upgrades of them, but while also handling conflict detection for
when a user has modified a mesh themselves.

Finally, a package can also contain *no* files, and just be empty with a
``metadata.yml``. This lets you create "meta" packages that don't do anything but
install a set of other packages. An example of where this might be useful is if
you packages each MQ2Nav mesh as it's own package, you could create meta packages
for each expansion, and then another metapackage above that for all nav mashes.


### How does this allow VV / OpenVanilla bundled packages to update?

Effectively VeryVanilla / OpenVanilla becomes a meta package as mentioned above,
an empty package that just contains dependencies on everything that is "shipped"
with VV itself.

A single .zip file containing everything could still be produced, it would just
contain the ``mqpkg.yml`` file and ``pkgdb`` directory in addition to what
it already ships.

Since under the covers, VV becomes just a meta package, that means all of the
individual components just becomes normal packages that can be managed
independently as well.


### How is Live vs Test vs Emu handled?

Strictly speaking, it's not handled. You would end up just treating these as
different compiles, each with their own repository where packages can be made
available at.

For packages that work "cross" compile (e.g. a macro, lua script, nav mesh, etc)
there's two strategies that could be employed:

1. Since the repository does not contain any packages itself, you could just
   duplicate the metadata into each repository, with the URLs pointing to the
   same files.
2. You could make another repository that contains all of the non compile specific
   packages, and have the common pattern to have like a ``live.json`` and ``all.json``
   repositories that are both added to the ``mqpkg.yml``.


### How are Paywalled packages handled?

The packaging tools do not care if a package is paywalled or not. There's two
strategies that can be used to implement them:

1. Make the metadata *not* paywalled (so name, version, dependencies, URLs) but
   the files themselves paywalled. The installer will be unaware that any of
   these packages it might not have access to, then when it attempts to download
   the actual packages, get an error for any package the user doesn't have access
   to.

2. Generate a repository for each paywalled package, and make it also paywalled
   (or not, you could still leave it open). Then when a user wants to add paywalled
   content, they have to first add a new line to their ``mqpkg.yml``, then
   after that everything functions as expected.


### How is the security of the packages managed?

The intent is that the package manifest should only be hosted over a valid HTTPS
connection, and installers should verify that. Package files do not have to be (but
can be) hosted over HTTPS, as the digest in the manifest will protect them regardless.


## Changes

- Require a ``.mq.zip`` extension. This will easily allow people to still use manual
  unpackaging of the package, while enabling us to more easily differentiate from any
  random zip file.

- Assume that some external system (in our case, RedGuides Resource Manager / Moderators)
  will ensure that only the correct people can publish packages for a particular package,
  the system itself will just assume the repository is correct.

- We're not going to use lzma or bzip2, since we're targeting making these easily able to
  be manually unpacked, and a lot of common tooling people are likely to be using to handle
  zip files likely won't be able to handle either.

- We're not going to do any zip inside zip tricks to get better compression. While these can
  produce smaller packages, they make it harder to unzip these manually, and we're trying
  to keep these something that can be easily manually installed. In addition, most of these
  packages aren't going to be very large to begin with, so the space savings is likely going
  to be pretty small.

- Unless someone comes up with something better, these packages are just going to be called
  MQ Packages.
