
CREATE DATABASE IF NOT EXISTS `lanka`;

USE `lanka`;
CREATE TABLE IF NOT EXISTS `boards` (
  `name` varchar(16) NOT NULL,
  `title` varchar(128) NOT NULL,
  `description` text DEFAULT NULL,
  `locked` bit(1) DEFAULT NULL,
  PRIMARY KEY (`name`)
);
CREATE TABLE IF NOT EXISTS `posts` (
  `id` int(16) unsigned NOT NULL,
  `board` char(16) NOT NULL,
  `thread` int(16) unsigned DEFAULT NULL,
  `content` mediumtext DEFAULT NULL,
  `attachments` text DEFAULT NULL,
  `reply` int(16) unsigned DEFAULT NULL,
  `bumped` timestamp NOT NULL DEFAULT current_timestamp(),
  `created` timestamp NOT NULL DEFAULT current_timestamp(),
  `author` varchar(32) DEFAULT NULL,
  `pinned` bit(1) DEFAULT NULL,
  `locked` bit(1) DEFAULT NULL,
  PRIMARY KEY (`id`),
  KEY `author` (`author`),
  KEY `board` (`board`),
  KEY `reply` (`reply`),
  KEY `thread` (`thread`),
  CONSTRAINT `2` FOREIGN KEY (`board`) REFERENCES `boards` (`name`) ON DELETE CASCADE ON UPDATE CASCADE,
  CONSTRAINT `3` FOREIGN KEY (`author`) REFERENCES `users` (`name`) ON DELETE CASCADE ON UPDATE CASCADE,
  CONSTRAINT `fk_posts_reply` FOREIGN KEY (`reply`) REFERENCES `posts` (`id`) ON DELETE CASCADE ON UPDATE CASCADE,
  CONSTRAINT `fk_posts_thread` FOREIGN KEY (`thread`) REFERENCES `posts` (`id`) ON DELETE CASCADE ON UPDATE CASCADE
);
CREATE TABLE IF NOT EXISTS `sessions` (
  `id` int(16) unsigned NOT NULL,
  `user` int(16) unsigned NOT NULL,
  `token_hash` varchar(32) NOT NULL,
  `created` timestamp NOT NULL DEFAULT current_timestamp(),
  `expires` timestamp NOT NULL DEFAULT (current_timestamp() + interval 7 day),
  `last_active` timestamp NOT NULL DEFAULT current_timestamp(),
  PRIMARY KEY (`id`),
  UNIQUE KEY `token_hash` (`token_hash`),
  KEY `user` (`user`),
  CONSTRAINT `1` FOREIGN KEY (`user`) REFERENCES `users` (`id`) ON DELETE CASCADE
);
CREATE TABLE IF NOT EXISTS `users` (
  `id` int(16) unsigned NOT NULL,
  `name` varchar(32) NOT NULL,
  `admin` bit(1) DEFAULT NULL,
  `password` varchar(256) NOT NULL,
  `email` varchar(64) NOT NULL,
  PRIMARY KEY (`id`),
  UNIQUE KEY `email` (`email`),
  UNIQUE KEY `name` (`name`)
);
