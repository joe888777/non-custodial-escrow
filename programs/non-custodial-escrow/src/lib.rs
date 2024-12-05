use anchor_lang::prelude::*;
use anchor_spl::token::{Mint, Token, TokenAccount };

// use macro transfer the string to publickey
declare_id!("F9cp6XU8hwEaM7hBgAXtURGkHTnGh6xWKKrQzLV4Prda");

#[program]
pub mod non_custodial_escrow {
    use super::*;

    //init escrow and store x token 
    pub fn initialize(ctx: Context<Initialize>, x_amount: u64, y_amount: u64) -> Result<()> {
        // new a escrow
        let escrow: &mut Account<Escrow> = &mut ctx.accounts.escrow;
        escrow.bump = ctx.bumps.escrow;
        escrow.authority = ctx.accounts.seller.key(); // authority 為 seller
        escrow.escrowed_x_tokens = ctx.accounts.escrowed_x_tokens.key(); // escrowed_x_tokens address
        escrow.y_amount = y_amount; // y amount
        escrow.y_mint = ctx.accounts.y_mint.key(); // mintY

        // transfer x token to seller_x_token address
        anchor_spl::token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                anchor_spl::token::Transfer {
                    from: ctx.accounts.seller_x_token.to_account_info(), // ATA of seller and mintX
                    to: ctx.accounts.escrowed_x_tokens.to_account_info(), // address used to save x token in escrow
                    authority: ctx.accounts.seller.to_account_info() //authority is seller
                },
            ),
            x_amount //transfer x amount x token
        )?;
        // in escrow, x amount is not record, but record y amount
        Ok(())
    }

    // buyer pay y token to get x token
    pub fn accept (ctx: Context<Accept>) -> Result<()> {
        anchor_spl::token::transfer(
            //when signing with PDA, use new_with_signer
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(), //
                anchor_spl::token::Transfer {
                    from: ctx.accounts.escrowed_x_tokens.to_account_info(),
                    to: ctx.accounts.buyer_x_token.to_account_info(),
                    authority: ctx.accounts.escrow.to_account_info()
                },
                &[&["escrow".as_bytes(), ctx.accounts.escrow.authority.as_ref(), &[ctx.accounts.escrow.bump]]]
            ),
            ctx.accounts.escrowed_x_tokens.amount
        )?;
        // transfer y token from buyer to seller's account
        anchor_spl::token::transfer(
            CpiContext::new(
                ctx.accounts.token_program.to_account_info(),
                anchor_spl::token::Transfer {
                    from: ctx.accounts.buyer_y_token.to_account_info(), // ATA of buyer & mintY
                    to: ctx.accounts.sellers_y_tokens.to_account_info(), // ATA of seller & mintY
                    authority: ctx.accounts.buyer.to_account_info() // authority is buyer's account
                }
            ),
            ctx.accounts.escrow.y_amount //y amount recorded in escrow
        )?;
        Ok(())
    }

    // cancel escrow and take x token
    pub fn cancel (ctx: Context<Cancel>) -> Result<()> {
        //return sellers x tokens for him/her
        anchor_spl::token::transfer(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(), //spl token_program *
                anchor_spl::token::Transfer{
                    from: ctx.accounts.escrowed_x_tokens.to_account_info(), //from escrowed_x_tokens
                    to: ctx.accounts.seller_x_token.to_account_info(), // ATA of seller & mintX
                    authority: ctx.accounts.escrow.to_account_info() //authority is escrow
                },
                &[&["escrow".as_bytes(), ctx.accounts.seller.key().as_ref(), &[ctx.accounts.escrow.bump]]] //sign seeds
            ),
            ctx.accounts.escrowed_x_tokens.amount
        )?;

        // close escrowed x token account and redeem sol
        anchor_spl::token::close_account(
            CpiContext::new_with_signer(
                ctx.accounts.token_program.to_account_info(),
                anchor_spl::token::CloseAccount {
                    account: ctx.accounts.escrowed_x_tokens.to_account_info(), // escrowed_x_tokens
                    destination: ctx.accounts.seller.to_account_info(), //redeem rent to seller
                    authority: ctx.accounts.escrow.to_account_info() // authority is escrow
                },
                &[&["escrow".as_bytes(), ctx.accounts.seller.key().as_ref(), &[ctx.accounts.escrow.bump]]]// signer seeds
            )
        )?;

        Ok(())
    }
}


#[derive(Accounts)]
pub struct Accept<'info> {
    pub buyer: Signer<'info>,
    #[account(
        mut,
        seeds = ["escrow".as_bytes(), escrow.authority.as_ref()],
        bump = escrow.bump
    )]
    pub escrow: Account<'info, Escrow>,
    #[account(mut, constraint = escrowed_x_tokens.key() == escrow.escrowed_x_tokens)]
    pub escrowed_x_tokens: Account<'info, TokenAccount>,
    #[account(mut, constraint = sellers_y_tokens.mint == escrow.y_mint)]
    pub sellers_y_tokens: Account<'info, TokenAccount>,
    #[account(mut, constraint = buyer_x_token.mint == escrowed_x_tokens.mint)]
    pub buyer_x_token: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = buyer_y_token.mint == escrow.y_mint,
        constraint = buyer_y_token.owner == buyer.key()
    )]
    pub buyer_y_token: Account<'info, TokenAccount>,
    pub token_program: Program<'info, Token>
}

#[derive(Accounts)]
pub struct Cancel<'info> {
    pub seller: Signer<'info>,
    #[account(
        mut,
        close = seller, constraint = escrow.authority == seller.key(),
        seeds = ["escrow".as_bytes(), escrow.authority.as_ref()],
        bump = escrow.bump
    )]
    pub escrow: Account<'info, Escrow>,
    #[account(mut, constraint = escrowed_x_tokens.key() == escrow.escrowed_x_tokens)]
    pub escrowed_x_tokens: Account<'info, TokenAccount>,
    #[account(
        mut,
        constraint = seller_x_token.mint == escrowed_x_tokens.mint,
        constraint = seller_x_token.owner == seller.key()
    )]
    pub seller_x_token: Account<'info, TokenAccount>,
    token_program: Program<'info, Token>
}

#[derive(Accounts)]
pub struct Initialize<'info>{
    #[account(mut)]
    seller: Signer<'info>,
    x_mint: Account<'info, Mint>,
    y_mint: Account<'info, Mint>,
    #[account(
        mut,
        constraint = 
            seller_x_token.mint == x_mint.key() &&
            seller_x_token.owner == seller.key()
    )]
    seller_x_token: Account<'info, TokenAccount>,
    #[account(
        init,
        payer = seller,
        space = Escrow::LEN,
        seeds = ["escrow".as_bytes(), seller.key().as_ref()],
        bump,
    )]
    pub escrow: Account<'info, Escrow>,
    #[account(
        init,
        payer = seller,
        token::mint = x_mint,
        token::authority = escrow
    )]
    escrowed_x_tokens: Account<'info, TokenAccount>,
    token_program: Program<'info, Token>,
    rent: Sysvar<'info, Rent>,
    system_program: Program<'info, System>
}

#[account]
pub struct Escrow {
    authority: Pubkey,
    bump: u8,
    escrowed_x_tokens: Pubkey,
    y_mint: Pubkey,
    y_amount: u64
}

impl Escrow {
    // First 8 Bytes are Discriminator (u64)
    pub const LEN: usize = 8 + 1 + 32 + 32 + 32 + 8;
}