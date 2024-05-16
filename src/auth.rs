use anyhow::{anyhow, bail, ensure, Context, Result};
use tracing::{debug, error, info, trace, warn};

use rand::Rng;
use std::{
    collections::HashMap,
    fs::File,
    path::{Path, PathBuf},
};

use borsh::{BorshDeserialize, BorshSerialize};
use cocoon::{Error, MiniCocoon};

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Clone, BorshDeserialize, BorshSerialize)]
pub struct Token {
    name: String,
    token: String,
    pub expiry: Expiration,
}

impl Token {
    pub fn get_token(&self) -> &str {
        &self.token
    }

    pub fn from_cookie(cookie: &cookie::Cookie) -> Result<Self> {
        let name = cookie.name().to_string();
        let token = cookie.value().to_string();
        let expiry = Expiration::from(cookie.expires().unwrap_or(cookie::Expiration::Session));

        Ok(Self {
            name,
            token,
            expiry,
        })
    }
}

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(Clone, BorshDeserialize, BorshSerialize)]
pub enum Expiration {
    Session,
    Timestamp(i64),
}

impl Expiration {
    pub fn expired(&self) -> bool {
        match self {
            Self::Session => false,
            Self::Timestamp(t) => {
                let now = chrono::Utc::now().timestamp();
                now > *t
            }
        }
    }
}

impl From<cookie::Expiration> for Expiration {
    fn from(value: cookie::Expiration) -> Self {
        match value {
            cookie::Expiration::DateTime(t) => Self::Timestamp(t.unix_timestamp()),
            cookie::Expiration::Session => Self::Session,
        }
    }
}

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct AuthInner {
    user: String,
    pass: String,
    token: Option<Token>,
    refresh_token: Option<Token>,
}

#[cfg_attr(debug_assertions, derive(Debug))]
#[derive(BorshDeserialize, BorshSerialize)]
pub struct Auth(Option<AuthInner>);

pub struct AuthDb {
    path: PathBuf,
    // file: File,
    // cocoon: Cocoon,
    // inner: MiniCocoon<Option<AuthDbInner>>,
    /// contains Option<AuthDbInner>
    cocoon: MiniCocoon,
}

impl AuthDb {
    pub fn read_or_create(path: &str) -> Result<Self> {
        let path: PathBuf = Path::new(path).to_path_buf();

        let key = std::env::var("COCOON_KEY")?;
        let seed = rand::thread_rng().gen::<[u8; 32]>();

        let mut cocoon = MiniCocoon::from_key(key.as_bytes(), &seed);

        if path.exists() {
            let mut file = std::fs::File::open(&path)?;

            let mut out = Self { path, cocoon };

            Ok(out)
        } else {
            let file = std::fs::File::create(&path)?;
            Ok(Self { path, cocoon })
        }
    }

    fn get_auth(&self) -> Result<Auth> {
        let mut file = std::fs::File::open(&self.path)?;
        let Ok(inner) = self.cocoon.parse(&mut file) else {
            bail!("Failed to decrypt auth file")
        };

        let Ok(inner) = Auth::try_from_slice(&inner) else {
            bail!("Failed to parse auth file")
        };

        Ok(inner)
    }

    fn set_auth(&mut self, mut auth: Auth) -> Result<()> {
        let Some(mut auth) = auth.0.take() else {
            return Ok(());
        };
        /// always overwrite?
        let mut file = std::fs::File::create(&self.path)?;

        if let Some(t) = &auth.token {
            if t.expiry.expired() {
                auth.token = None;
            }
        }
        if let Some(t) = &auth.refresh_token {
            if t.expiry.expired() {
                auth.refresh_token = None;
            }
        }

        let auth = Auth(Some(auth));

        let encoded = borsh::to_vec(&auth)?;
        let Ok(_) = self.cocoon.dump(encoded, &mut file) else {
            bail!("Failed to encrypt auth file")
        };
        Ok(())
    }

    fn set_credentials(&mut self, username: &str, pass: &str) -> Result<()> {
        let auth = AuthInner {
            user: username.to_string(),
            pass: pass.to_string(),
            token: None,
            refresh_token: None,
        };

        self.set_auth(Auth(Some(auth)))?;

        Ok(())
    }

    fn set_token(&mut self, token: Option<Token>, refresh: bool) -> Result<()> {
        if let Some(mut auth) = self.get_auth()?.0 {
            if refresh {
                auth.refresh_token = token;
            } else {
                auth.token = token;
            }
            self.set_auth(Auth(Some(auth)))?;
        } else {
            bail!("No credentials set")
        };
        Ok(())
    }

    pub fn get_token(&self) -> Result<Option<Token>> {
        let auth = self.get_auth()?;
        if let Some(token) = auth.0.and_then(|a| a.token) {
            if token.expiry.expired() {
                Ok(None)
            } else {
                Ok(Some(token))
            }
        } else {
            Ok(None)
        }
    }

    pub fn login_and_get_token(&mut self, username: &str, pass: &str) -> Result<Token> {
        self.set_credentials(username, pass)?;

        const URL: &'static str = "https://bambulab.com/api/sign-in/form";

        let mut map = HashMap::new();
        map.insert("account", username);
        map.insert("password", pass);
        // map.insert("apiError", "");

        let client = reqwest::blocking::ClientBuilder::new()
            .use_rustls_tls()
            .build()?;
        let res = client.post(URL).json(&map).send()?;

        if !res.status().is_success() {
            bail!("Failed to login")
        }

        let cookies = res.headers().get_all("set-cookie");

        let mut token = None;
        let mut refresh_token = None;
        let mut token_expires = None;
        let mut refresh_token_expires = None;

        for cookie in cookies.iter() {
            let cookie = cookie::Cookie::parse(cookie.to_str()?).unwrap();

            if cookie.name() == "token" {
                debug!("expires = {:?}", cookie.expires());
                token = Some(Token::from_cookie(&cookie)?);
            } else if cookie.name() == "expiresIn" {
                token_expires = Some(cookie.value().parse()?);
            } else if cookie.name() == "refreshExpiresIn" {
                refresh_token_expires = Some(cookie.value().parse()?);
            } else if cookie.name() == "refreshToken" {
                refresh_token = Some(Token::from_cookie(&cookie)?);
            }
        }

        let mut token = token.context("Failed to get token")?;
        let expires = token_expires.unwrap();
        let t = chrono::Utc::now() + chrono::TimeDelta::new(expires, 0).unwrap();
        token.expiry = Expiration::Timestamp(t.timestamp());

        let mut refresh_token = refresh_token.context("Failed to get refresh token")?;
        let expires = refresh_token_expires.unwrap();
        let t = chrono::Utc::now() + chrono::TimeDelta::new(expires, 0).unwrap();
        refresh_token.expiry = Expiration::Timestamp(t.timestamp());

        self.set_token(Some(token.clone()), false)?;

        self.set_token(Some(refresh_token), true)?;

        Ok(token)
    }

    //
}
