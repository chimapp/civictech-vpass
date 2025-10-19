-- Add wallet_qr_data field to membership_cards for Taiwan Digital Wallet QR code
ALTER TABLE membership_cards
ADD COLUMN wallet_qr_data TEXT;
