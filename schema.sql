
CREATE DATABASE IF NOT EXISTS `lanka`;

USE `lanka`;
CREATE TABLE IF NOT EXISTS `attachments` (
  `id` int(16) unsigned NOT NULL,
  `name` varchar(64) NOT NULL,
  `post` int(16) unsigned DEFAULT NULL,
  `size` int(16) unsigned NOT NULL,
  `original_name` varchar(256) NOT NULL,
  PRIMARY KEY (`id`),
  KEY `fk_attachments_post` (`post`),
  CONSTRAINT `fk_attachments_post` FOREIGN KEY (`post`) REFERENCES `posts` (`id`) ON DELETE SET NULL ON UPDATE CASCADE
);
CREATE TABLE IF NOT EXISTS `boards` (
  `name` varchar(16) NOT NULL,
  `theme` varchar(128) NOT NULL,
  `title` varchar(128) NOT NULL,
  `description` text DEFAULT NULL,
  `locked` bit(1) DEFAULT NULL,
  PRIMARY KEY (`name`),
  KEY `theme` (`theme`),
  CONSTRAINT `fk_boards_theme` FOREIGN KEY (`theme`) REFERENCES `themes` (`name`) ON DELETE CASCADE ON UPDATE CASCADE
);
CREATE TABLE IF NOT EXISTS `posts` (
  `id` int(16) unsigned NOT NULL,
  `board` char(16) NOT NULL,
  `thread` int(16) unsigned DEFAULT NULL,
  `reply` int(16) unsigned DEFAULT NULL,
  `content` mediumtext DEFAULT NULL,
  `author` varchar(32) DEFAULT NULL,
  `created` timestamp NOT NULL DEFAULT current_timestamp(),
  `bumped` timestamp NOT NULL DEFAULT current_timestamp(),
  `pinned` bit(1) DEFAULT NULL,
  `locked` bit(1) DEFAULT NULL,
  PRIMARY KEY (`id`),
  KEY `author` (`author`),
  KEY `board` (`board`),
  KEY `reply` (`reply`),
  KEY `thread` (`thread`),
  CONSTRAINT `2` FOREIGN KEY (`board`) REFERENCES `boards` (`name`) ON DELETE CASCADE ON UPDATE CASCADE,
  CONSTRAINT `3` FOREIGN KEY (`author`) REFERENCES `users` (`name`) ON DELETE CASCADE ON UPDATE CASCADE,
  CONSTRAINT `4` FOREIGN KEY (`reply`) REFERENCES `posts` (`id`) ON DELETE SET NULL ON UPDATE CASCADE,
  CONSTRAINT `fk_posts_thread` FOREIGN KEY (`thread`) REFERENCES `posts` (`id`) ON DELETE CASCADE ON UPDATE CASCADE
);
CREATE TABLE IF NOT EXISTS `sessions` (
  `id` int(16) unsigned NOT NULL,
  `user` int(16) unsigned NOT NULL,
  `token_hash` char(64) NOT NULL,
  `created` timestamp NOT NULL DEFAULT current_timestamp(),
  `expires` timestamp NOT NULL DEFAULT (current_timestamp() + interval 7 day),
  `last_active` timestamp NOT NULL DEFAULT current_timestamp(),
  PRIMARY KEY (`id`),
  UNIQUE KEY `token_hash` (`token_hash`),
  KEY `user` (`user`),
  CONSTRAINT `1` FOREIGN KEY (`user`) REFERENCES `users` (`id`) ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS `themes` (
  `name` varchar(128) NOT NULL,
  PRIMARY KEY (`name`)
);
CREATE TABLE IF NOT EXISTS `users` (
  `id` int(16) unsigned NOT NULL,
  `name` varchar(32) NOT NULL,
  `admin` bit(1) NOT NULL DEFAULT b'0',
  `password` char(64) NOT NULL,
  `email` varchar(64) NOT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `email` (`email`),
  UNIQUE KEY `name` (`name`)
);
