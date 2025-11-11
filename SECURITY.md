# Security Policy

## Reporting a Vulnerability

The security of our stake pool program is a top priority. We appreciate the security community's efforts in responsibly disclosing vulnerabilities.

### How to Report

If you discover a security vulnerability, please report it to us privately:

**Email:** hello@yourwallet.tr

Please include the following information in your report:
- Description of the vulnerability
- Steps to reproduce the issue
- Potential impact
- Suggested fix (if any)

### What to Expect

- **Acknowledgment:** We will acknowledge receipt of your report within 48 hours
- **Assessment:** We will assess the vulnerability and determine its severity
- **Updates:** We will keep you informed about our progress in addressing the issue
- **Resolution:** We will work to fix confirmed vulnerabilities as quickly as possible
- **Disclosure:** We will coordinate with you on the timing of public disclosure

### Security Best Practices

When using this stake pool program:

1. **Audit Smart Contracts:** Always review and audit the program code before deploying to mainnet
2. **Test Thoroughly:** Use devnet and testnet environments for testing before mainnet deployment
3. **Key Management:** Secure your keypairs and never expose private keys
4. **Monitor Activity:** Regularly monitor stake pool operations and transactions
5. **Stay Updated:** Keep up with security advisories and program updates

### Scope

This security policy applies to:
- The stake pool Solana program (`program/`)
- JavaScript/TypeScript client libraries (`clients/js/`)
- Example implementations (`example/`)

### Out of Scope

- Third-party dependencies (please report to respective maintainers)
- Issues in test environments that don't affect production

## Security Audit

For information about security audits performed on this project, please see [SECURITY_AUDIT.md](audit/SECURITY_AUDIT.md).

## Responsible Disclosure

We follow responsible disclosure practices and request that you:
- Do not publicly disclose the vulnerability until we have had a chance to address it
- Do not exploit the vulnerability beyond what is necessary to demonstrate it
- Make a good faith effort to avoid privacy violations, data destruction, and service interruption

Thank you for helping keep our stake pool program and our users safe!
