## Security Vulnerabilities

<blockquote style="background: rgba(255, 200, 0, 0.1); border: 5px solid rgba(255, 200, 0, 0.4);">

The intended audience of the information presented here is developers working
on the implementation of UNC.

Are you a security researcher? Please report security vulnerabilities to
[security@unc.org](mailto:security@unc.org).

</blockquote>

As framework is open source, all of its issues and pull requests are also
publicly tracked on GitHub. However, from time to time, if a security-sensitive
issue is discovered, it cannot be tracked publicly on GitHub. However, we
should promote as similar a development process to work on such issues as
possible. To enable this, below is the high-level process for working on
security-sensitive issues.

1. There is a [private fork of
   framework](https://github.com/utnet-org/utility-private) on GitHub. Access to
   this repository is restricted to the set of people who are trusted to work on
   and have knowledge about security-sensitive issues in framework.

   This repository can be manually synced with the public framework repository
   using the following commands:

    ```console
    $ git remote add framework-public git@github.com:utnet-org/utility
    $ git remote add framework-private git@github.com:utnet-org/utility-private
    $ git fetch framework-public
    $ git push framework-private framework-public/master:master
    ```
2. All security-sensitive issues must be created on the private framework
   repository. You must also assign one of the `[P-S0, P-S1]` labels to the
   issue to indicate the severity of the issue. The two criteria to use to help
   you judge the severity are the ease of carrying out the attack and the impact
   of the attack. An attack that is easy to do or can have a huge impact should
   have the `P-S0` label and `P-S1` otherwise.

3. All security-sensitive pull requests should also be created on the private
   framework repository. Note that once a PR has been approved, it should not be
   merged into the private repository. Instead, it should be first merged into
   the public repository and then the private fork should be updated using the
   steps above.

4. Once work on a security issue is finished, it needs to be deployed to all the
   impacted networks. Please contact the node team for help with this.
