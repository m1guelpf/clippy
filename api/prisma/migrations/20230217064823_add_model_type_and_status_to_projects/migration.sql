-- AlterTable
ALTER TABLE `Project` ADD COLUMN `modelType` ENUM('Metal', 'Plastic') NOT NULL DEFAULT 'Metal',
    ADD COLUMN `status` ENUM('Pending', 'Trained') NOT NULL DEFAULT 'Pending';
